#!/usr/bin/env python3
"""Opt-in Kubernetes provider for the ignored Phase-0 Factorio fixture tests.

This executable is intentionally outside Cargo's normal test path. It uses only
kubectl plus the Python standard library, creates one disposable namespace per
invocation, and removes it on every exit path.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parent.parent
MOD_ROOT = ROOT / "mods" / "factorio-agent-bridge"
IMAGE = os.environ.get("FACTORIO_FIXTURE_IMAGE", "factoriotools/factorio:2.1.17")
TIMEOUT_SECONDS = int(os.environ.get("FACTORIO_FIXTURE_TIMEOUT_SECONDS", "240"))
# The bundled RCON client opens one private connection per command. Keep polling
# below the server's login-rate guard while allowing a zero-client game to tick.
ACTION_POLL_SECONDS = 0.75


def fail(message: str) -> "NoReturn":
    raise RuntimeError(message)


def command(args: list[str], *, input_text: str | None = None, timeout: int = TIMEOUT_SECONDS) -> str:
    completed = subprocess.run(
        args,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if completed.returncode:
        detail = completed.stderr.strip() or completed.stdout.strip()
        fail(f"command failed ({' '.join(args[:4])}): {detail}")
    return completed.stdout


def kubectl(namespace: str | None, *args: str, input_text: str | None = None, timeout: int = TIMEOUT_SECONDS) -> str:
    invocation = ["kubectl"]
    if namespace:
        invocation.extend(["--namespace", namespace])
    invocation.extend(args)
    return command(invocation, input_text=input_text, timeout=timeout)


def package_bridge(target: Path) -> None:
    if not MOD_ROOT.is_dir():
        fail(f"bridge mod source is missing: {MOD_ROOT}")
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(MOD_ROOT.rglob("*")):
            if path.is_file():
                archive.write(path, f"factorio-agent-bridge_0.3.0/{path.relative_to(MOD_ROOT).as_posix()}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_fixture_server_settings(path: Path) -> None:
    """Keep the zero-client fixture simulating without an attached human."""
    settings = {
        "name": "Factorio Agent Bridge disposable fixture",
        "description": "",
        "tags": [],
        "max_players": 0,
        "visibility": {"public": False, "lan": False},
        "username": "",
        "password": "",
        "token": "",
        "game_password": "",
        "require_user_verification": False,
        "max_upload_in_kilobytes_per_second": 0,
        "max_upload_slots": 5,
        "minimum_latency_in_ticks": 0,
        "max_heartbeats_per_second": 60,
        "ignore_player_limit_for_returning_players": False,
        "allow_commands": "admins-only",
        "autosave_interval": 10,
        "autosave_slots": 5,
        "afk_autokick_interval": 0,
        "auto_pause": False,
        "auto_pause_when_players_connect": False,
        "only_admins_can_pause_the_game": True,
        "autosave_only_on_server": True,
        "non_blocking_saving": False,
        "minimum_segment_size": 25,
        "minimum_segment_size_peer_count": 20,
        "maximum_segment_size": 100,
        "maximum_segment_size_peer_count": 10,
    }
    path.write_text(json.dumps(settings, indent=2) + "\n", encoding="utf-8")


def manifest(namespace: str) -> str:
    labels = {"app.kubernetes.io/name": "factorio-agent-bridge", "app.kubernetes.io/component": "phase-0-fixture"}
    return json.dumps(
        {
            "apiVersion": "v1",
            "kind": "List",
            "items": [
                {
                    "apiVersion": "v1",
                    "kind": "Namespace",
                    "metadata": {"name": namespace, "labels": labels},
                },
                {
                    "apiVersion": "v1",
                    "kind": "PersistentVolumeClaim",
                    "metadata": {"name": "factorio-data", "namespace": namespace, "labels": labels},
                    "spec": {"accessModes": ["ReadWriteOnce"], "resources": {"requests": {"storage": "2Gi"}}},
                },
                {
                    "apiVersion": "v1",
                    "kind": "Pod",
                    "metadata": {"name": "save-stager", "namespace": namespace, "labels": labels},
                    "spec": {
                        "restartPolicy": "Never",
                        "containers": [{"name": "stager", "image": "busybox:1.37.0", "command": ["sh", "-ec", "mkdir -p /fixture/saves; sleep 600"], "volumeMounts": [{"name": "data", "mountPath": "/fixture"}]}],
                        "volumes": [{"name": "data", "persistentVolumeClaim": {"claimName": "factorio-data"}}],
                    },
                },
            ],
        }
    )


def server_manifest(namespace: str) -> str:
    labels = {"app.kubernetes.io/name": "factorio-agent-bridge", "app.kubernetes.io/component": "phase-0-fixture"}
    return json.dumps(
        {
            "apiVersion": "v1",
            "kind": "List",
            "items": [
                {
                    "apiVersion": "v1",
                    "kind": "Service",
                    "metadata": {"name": "factorio-rcon", "namespace": namespace, "labels": labels},
                    "spec": {"type": "ClusterIP", "selector": {"app": "factorio-fixture"}, "ports": [{"name": "rcon", "port": 27015, "targetPort": 27015, "protocol": "TCP"}]},
                },
                {
                    "apiVersion": "apps/v1",
                    "kind": "Deployment",
                    "metadata": {"name": "factorio-fixture", "namespace": namespace, "labels": labels},
                    "spec": {
                        "replicas": 1,
                        "strategy": {"type": "Recreate"},
                        "selector": {"matchLabels": {"app": "factorio-fixture"}},
                        "template": {
                            "metadata": {"labels": {"app": "factorio-fixture", **labels}},
                            "spec": {
                                "initContainers": [{
                                    "name": "install-fixture-inputs",
                                    "image": "busybox:1.37.0",
                                    "command": ["sh", "-ec", "set -eu; mkdir -p /factorio/config /factorio/mods /factorio/saves; cp /source/bridge.zip /factorio/mods/factorio-agent-bridge_0.3.0.zip; cp /source/mod-list.json /factorio/mods/mod-list.json; cp /source/server-settings.json /factorio/config/server-settings.json; cp /secret/rconpw /factorio/config/rconpw; chmod 600 /factorio/config/rconpw"],
                                    "volumeMounts": [{"name": "data", "mountPath": "/factorio"}, {"name": "bridge", "mountPath": "/source", "readOnly": True}, {"name": "rcon", "mountPath": "/secret", "readOnly": True}],
                                }],
                                "containers": [{
                                    "name": "factorio",
                                    "image": IMAGE,
                                    "imagePullPolicy": "IfNotPresent",
                                    "env": [{"name": "GENERATE_NEW_SAVE", "value": "false"}, {"name": "LOAD_LATEST_SAVE", "value": "false"}, {"name": "SAVE_NAME", "value": "fixture"}, {"name": "DLC_SPACE_AGE", "value": "false"}],
                                    "ports": [{"name": "rcon", "containerPort": 27015, "protocol": "TCP"}],
                                    "resources": {"requests": {"cpu": "500m", "memory": "1Gi"}, "limits": {"cpu": "2", "memory": "2Gi"}},
                                    "volumeMounts": [{"name": "data", "mountPath": "/factorio"}],
                                }],
                                "volumes": [{"name": "data", "persistentVolumeClaim": {"claimName": "factorio-data"}}, {"name": "bridge", "configMap": {"name": "bridge-inputs"}}, {"name": "rcon", "secret": {"secretName": "rcon-password"}}],
                            },
                        },
                    },
                },
            ],
        }
    )


def wait_for_pod(namespace: str) -> str:
    kubectl(namespace, "wait", "--for=condition=Ready", "pod", "-l", "app=factorio-fixture", f"--timeout={TIMEOUT_SECONDS}s")
    return kubectl(namespace, "get", "pods", "-l", "app=factorio-fixture", "-o", "jsonpath={.items[0].metadata.name}").strip()


def fixed_query(namespace: str, pod: str, request: str) -> dict[str, Any]:
    # The runner is test-only but remains a closed dispatch: callers cannot supply
    # Lua, an RCON command, a remote interface, or a bridge method.
    requests = {
        "contract": '{name="get_capability_contract"}',
        "status": '{name="get_actor_status"}',
        "local": '{name="scan_local",radius=1}',

        "maximum": '{name="scan_local",radius=32}',
        "over_limit": '{name="scan_local",radius=33}',
        "uncharted": '{name="scan_charted",center={x=1000000,y=1000000},radius=0}',
    }
    if request not in requests:
        fail(f"unknown fixed fixture query: {request}")
    lua = f'/c rcon.print(helpers.table_to_json(remote.call("factorio_agent_bridge","query",{requests[request]})))'
    # Factorio requires confirmation before the first console command in a save.
    # The first fixed command grants that confirmation; the identical second call
    # yields the bridge response. This occurs only in the disposable fixture.
    kubectl(namespace, "exec", pod, "--", "rcon", lua)
    output = kubectl(namespace, "exec", pod, "--", "rcon", lua)
    for line in reversed(output.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    fail(f"fixed bridge query returned no JSON for {request}")
    raise AssertionError("unreachable")


def fixed_action(namespace: str, pod: str, action: str, *, target: dict[str, float] | None = None) -> dict[str, Any]:
    """Call only the fixed Phase-1 action probe vocabulary.

    Target coordinates derive solely from the preceding bridge status. The runner
    accepts no caller-supplied Lua, action ID, interface, or method name.
    """
    if action == "walk_success":
        assert target is not None
        request = f'{{name="walk_to",action_id="fixture-walk-success",target={{x={target["x"]:.6f},y={target["y"]:.6f}}}}}'
    elif action == "walk_stop":
        assert target is not None
        request = f'{{name="walk_to",action_id="fixture-walk-stop",target={{x={target["x"]:.6f},y={target["y"]:.6f}}}}}'
    elif action == "stop":
        request = '{name="stop",action_id="fixture-stop"}'
    elif action == "get_success":
        request = '{name="get_action",action_id="fixture-walk-success"}'
    elif action == "get_stop":
        request = '{name="get_action",action_id="fixture-walk-stop"}'
    elif action == "mine_approach_one":
        request = '{name="walk_to",action_id="fixture-mine-approach-one",target={x=-24,y=-24}}'
    elif action == "get_mine_approach_one":
        request = '{name="get_action",action_id="fixture-mine-approach-one"}'
    elif action == "mine_approach_two":
        request = '{name="walk_to",action_id="fixture-mine-approach-two",target={x=-44,y=-46}}'
    elif action == "get_mine_approach_two":
        request = '{name="get_action",action_id="fixture-mine-approach-two"}'
    elif action == "mine_out_of_reach":
        request = '{name="mine",action_id="fixture-mine-out-of-reach",target={x=-46.5,y=-44.5}}'
    elif action == "mine":
        request = '{name="mine",action_id="fixture-mine",target={x=-45.5,y=-45.5}}'
    elif action == "get_mine":
        request = '{name="get_action",action_id="fixture-mine"}'
    elif action == "mine_conflict":
        request = '{name="mine",action_id="fixture-mine",target={x=-45.5,y=-44.5}}'
    elif action == "mine_cancel":
        request = '{name="mine",action_id="fixture-mine-cancel",target={x=-45.5,y=-44.5}}'
    elif action == "get_mine_cancel":
        request = '{name="get_action",action_id="fixture-mine-cancel"}'
    elif action == "mine_stop":
        request = '{name="stop",action_id="fixture-mine-stop"}'
    else:
        fail(f"unknown fixed fixture action: {action}")
    lua = f'/c rcon.print(helpers.table_to_json(remote.call("factorio_agent_bridge","command",{request})))'
    output = kubectl(namespace, "exec", pod, "--", "rcon", lua)
    for line in reversed(output.splitlines()):
        line = line.strip()
        if line.startswith("{"):
            return json.loads(line)
    fail(f"fixed bridge action returned no JSON for {action}")
    raise AssertionError("unreachable")


def assert_ok(response: dict[str, Any], name: str) -> dict[str, Any]:
    if response.get("ok") is not True or not isinstance(response.get("result"), dict):
        fail(f"{name} bridge query was rejected: {response.get('error', {})}")
    return response["result"]


def collect_logs(namespace: str, pod: str, report: Path) -> None:
    report.mkdir(parents=True, exist_ok=True)
    for label, args in {"factorio.log": ["logs", pod, "-c", "factorio"], "pod.json": ["get", "pod", pod, "-o", "json"]}.items():
        try:
            content = kubectl(namespace, *args)
            # The stock container entrypoint echoes its RCON invocation. Fixture
            # evidence must never retain its randomly generated secret.
            if label == "factorio.log":
                content = re.sub(r"(--rcon-password\s+)(\S+)", r"\1<redacted>", content)
            (report / label).write_text(content, encoding="utf-8")
        except RuntimeError as error:
            (report / f"{label}.error").write_text(str(error), encoding="utf-8")


def run_lifecycle(namespace: str, pod: str, report: dict[str, Any]) -> None:
    before = assert_ok(fixed_query(namespace, pod, "status"), "initial actor status")
    report["initial_status"] = before
    kubectl(namespace, "delete", "pod", pod, "--wait=true", f"--timeout={TIMEOUT_SECONDS}s")
    restarted = wait_for_pod(namespace)

    after = assert_ok(fixed_query(namespace, restarted, "status"), "restarted actor status")
    report["restarted_status"] = after
    if before["actor_id"] != after["actor_id"] or before["unit_number"] != after["unit_number"]:
        fail("actor identity changed across server restart")


def run_observation(namespace: str, pod: str, report: dict[str, Any], args: argparse.Namespace) -> None:
    contract = assert_ok(fixed_query(namespace, pod, "contract"), "capability contract")
    limits = contract["observation_limits"]
    if limits != {"max_radius": args.assert_local_radius, "max_result_count": args.assert_max_results, "max_detail_fields": 4, "max_payload_bytes": args.assert_max_payload_bytes}:
        fail(f"bridge observation limits differ from fixture contract: {limits}")
    local = assert_ok(fixed_query(namespace, pod, "local"), "local observation")
    report["local_observation"] = local
    if local["effective_bounds"]["radius"] != 1 or local["result_count"] > args.assert_max_results or local["payload_bytes"] > args.assert_max_payload_bytes:
        fail("local observation exceeded declared bounds")
    maximum = assert_ok(fixed_query(namespace, pod, "maximum"), "maximum local observation")
    report["maximum_local_observation"] = maximum
    if maximum["effective_bounds"]["radius"] != args.assert_local_radius or maximum["result_count"] > args.assert_max_results or maximum["payload_bytes"] > args.assert_max_payload_bytes:
        fail("maximum local observation exceeded declared bounds")
    over_limit = fixed_query(namespace, pod, "over_limit")
    report["over_limit_observation"] = over_limit
    if over_limit.get("ok") is not False or over_limit.get("error", {}).get("code") != "OUT_OF_POLICY":
        fail("radius above the declared local bound was not rejected")
    denied = fixed_query(namespace, pod, "uncharted")
    report["uncharted_observation"] = denied
    if denied.get("ok") is not False or denied.get("error", {}).get("code") != "UNCHARTED":
        fail("uncharted observation was not denied before lookup")


def wait_for_action(namespace: str, pod: str, action: str) -> dict[str, Any]:
    deadline = time.monotonic() + TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        result = assert_ok(fixed_action(namespace, pod, action), action)
        if result.get("state") in {"succeeded", "failed", "cancelled", "rejected"}:
            return result
        time.sleep(ACTION_POLL_SECONDS)
    fail(f"Phase-1 action {action} did not reach a terminal state")
    raise AssertionError("unreachable")


def run_walk_stop(namespace: str, pod: str, report: dict[str, Any]) -> None:
    before = assert_ok(fixed_query(namespace, pod, "status"), "walk initial status")
    start = before["position"]
    accepted = assert_ok(fixed_action(namespace, pod, "walk_success", target={"x": start["x"] + 3.0, "y": start["y"]}), "walk acceptance")
    if accepted.get("state") != "accepted":
        fail(f"walk was not accepted: {accepted}")
    completed = wait_for_action(namespace, pod, "get_success")
    report["walk"] = completed
    if completed.get("state") != "succeeded" or completed.get("result", {}).get("code") != "TARGET_REACHED":
        fail(f"headless walk did not reach the target: {completed}")
    if not isinstance(completed.get("start_tick"), int) or not isinstance(completed.get("end_tick"), int) or completed["end_tick"] <= completed["start_tick"]:
        fail("walk receipt does not demonstrate elapsed game ticks")

    end = completed["end_position"]
    assert_ok(fixed_action(namespace, pod, "walk_stop", target={"x": end["x"] + 32.0, "y": end["y"]}), "stop walk acceptance")
    stopped = assert_ok(fixed_action(namespace, pod, "stop"), "stop receipt")
    cancelled = wait_for_action(namespace, pod, "get_stop")
    report["stop"] = {"receipt": stopped, "cancelled_walk": cancelled}
    if stopped.get("state") != "succeeded" or stopped.get("result", {}).get("code") != "STOPPED":
        fail(f"stop did not produce a successful receipt: {stopped}")
    if cancelled.get("state") != "cancelled" or cancelled.get("result", {}).get("code") != "STOP_REQUESTED":
        fail(f"stop did not cancel the active walk: {cancelled}")


def run_mine(namespace: str, pod: str, report: dict[str, Any]) -> str:
    """Exercise a fixed, copied-save mining route without arbitrary RCON input."""
    initial = assert_ok(fixed_query(namespace, pod, "status"), "mining initial status")
    report["initial_status"] = initial

    # These are fixed fixture-only action IDs and targets for the retained seed
    # save's iron patch; callers cannot select arbitrary locations or Lua.
    route = (
        ("mine_approach_one", "get_mine_approach_one"),
        ("mine_approach_two", "get_mine_approach_two"),
    )
    report["mine_approach"] = []
    for action, get_action in route:
        accepted_result = assert_ok(fixed_action(namespace, pod, action), f"{action} acceptance")
        if accepted_result.get("state") != "accepted":
            fail(f"{action} was not accepted: {accepted_result}")
        result = wait_for_action(namespace, pod, get_action)
        if result.get("state") != "succeeded" or result.get("result", {}).get("code") != "TARGET_REACHED":
            fail(f"{action} did not reach its fixed target: {result}")
        report["mine_approach"].append(result)

    out_of_reach = fixed_action(namespace, pod, "mine_out_of_reach")
    if out_of_reach.get("ok") is not False or out_of_reach.get("error", {}).get("code") != "OUT_OF_POLICY":
        fail("out-of-reach mine request was not rejected")

    accepted = assert_ok(fixed_action(namespace, pod, "mine"), "mine acceptance")
    if accepted.get("state") != "accepted":
        fail(f"mine was not accepted: {accepted}")
    mined = wait_for_action(namespace, pod, "get_mine")
    if mined.get("state") != "succeeded" or mined.get("result", {}).get("code") != "MINED":
        fail(f"mine did not complete through ordinary character input: {mined}")
    if mined.get("end_tick", 0) <= mined.get("start_tick", 0):
        fail("mining receipt does not demonstrate elapsed game ticks")
    if mined.get("inventory_delta") != [{"name": "iron-ore", "quality": "normal", "count": 1}]:
        fail(f"mine receipt did not conserve exactly one iron ore: {mined}")
    before_amount = mined.get("target_amount_before")
    after_amount = mined.get("target_amount_after")
    if not isinstance(before_amount, int) or after_amount != before_amount - 1:
        fail(f"mine receipt did not prove one-unit target depletion: {before_amount} -> {after_amount}")

    retry = assert_ok(fixed_action(namespace, pod, "mine"), "idempotent mine retry")
    if retry != mined:
        fail("identical mine retry did not return the original completed receipt")
    conflict = fixed_action(namespace, pod, "mine_conflict")
    if conflict.get("ok") is not False or conflict.get("error", {}).get("code") != "ACTION_ID_CONFLICT":
        fail("conflicting mine action ID was not rejected")


    assert_ok(fixed_action(namespace, pod, "mine_cancel"), "mine cancellation acceptance")
    stopped = assert_ok(fixed_action(namespace, pod, "mine_stop"), "mine stop receipt")
    cancelled = wait_for_action(namespace, pod, "get_mine_cancel")
    if stopped.get("result", {}).get("code") != "STOPPED" or cancelled.get("state") != "cancelled" or cancelled.get("result", {}).get("code") != "STOP_REQUESTED":
        fail(f"stop did not cancel active mining: stop={stopped}, mine={cancelled}")

    report["mine"] = {
        "out_of_reach": out_of_reach,
        "before_amount": before_amount,
        "receipt": mined,
        "retry": retry,
        "conflict": conflict,
        "after_amount": after_amount,
        "stop": stopped,
        "cancelled": cancelled,
    }
    return pod


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--save", required=True, type=Path, help="copied disposable Factorio save")
    parser.add_argument("--mod-settings", type=Path)
    parser.add_argument("--assert-local-radius", type=int)
    parser.add_argument("--assert-max-results", type=int)
    parser.add_argument("--assert-max-payload-bytes", type=int)
    parser.add_argument("--assert-walk", action="store_true")
    parser.add_argument("--assert-mine", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if shutil.which("kubectl") is None:
        fail("kubectl is required for the Kubernetes fixture provider")
    if not args.save.is_file():
        fail(f"disposable save does not exist: {args.save}")
    lifecycle = args.mod_settings is not None
    observation = args.assert_local_radius is not None
    walk = args.assert_walk
    mine = args.assert_mine
    if sum((lifecycle, observation, walk, mine)) != 1:
        fail("runner invocation must be exactly one of lifecycle, observation, walk/stop, or mine")
    if lifecycle and not args.mod_settings.is_file():
        fail(f"mod-settings fixture does not exist: {args.mod_settings}")
    if observation and (args.assert_max_results is None or args.assert_max_payload_bytes is None):
        fail("observation fixture requires all asserted limits")

    namespace = f"factorio-fixture-{secrets.token_hex(4)}"
    report_dir = ROOT / "test-reports" / namespace
    mode = "lifecycle" if lifecycle else "observation" if observation else "walk-stop" if walk else "mine"
    report: dict[str, Any] = {"namespace": namespace, "input_save_sha256": sha256(args.save), "mode": mode, "image": IMAGE}
    pod: str | None = None
    with tempfile.TemporaryDirectory(prefix="factorio-fixture-") as temporary:
        temporary_path = Path(temporary)
        archive = temporary_path / "bridge.zip"
        mod_list = temporary_path / "mod-list.json"
        server_settings = temporary_path / "server-settings.json"
        package_bridge(archive)
        mod_list.write_text('{"mods":[{"name":"base","enabled":true},{"name":"factorio-agent-bridge","enabled":true}]}\n', encoding="utf-8")
        write_fixture_server_settings(server_settings)
        password = secrets.token_urlsafe(32)
        try:
            kubectl(None, "apply", "-f", "-", input_text=manifest(namespace))
            kubectl(namespace, "wait", "--for=condition=Ready", "pod/save-stager", f"--timeout={TIMEOUT_SECONDS}s")
            kubectl(namespace, "cp", str(args.save), "save-stager:/fixture/saves/fixture.zip")
            kubectl(namespace, "delete", "pod", "save-stager", "--wait=true", f"--timeout={TIMEOUT_SECONDS}s")
            kubectl(namespace, "create", "configmap", "bridge-inputs", f"--from-file=bridge.zip={archive}", f"--from-file=mod-list.json={mod_list}", f"--from-file=server-settings.json={server_settings}")
            kubectl(namespace, "create", "secret", "generic", "rcon-password", f"--from-literal=rconpw={password}")
            kubectl(None, "apply", "-f", "-", input_text=server_manifest(namespace))
            pod = wait_for_pod(namespace)
            # The copied save retains its previous paused state. This changes only
            # server time flow in the disposable zero-client fixture; it does not
            # create resources, alter inventories, move actors, or complete actions.
            kubectl(namespace, "exec", pod, "--", "rcon", "/c game.tick_paused=false")
            kubectl(namespace, "exec", pod, "--", "rcon", "/c game.tick_paused=false")
            report["pause_probe_before"] = kubectl(namespace, "exec", pod, "--", "rcon", "/c rcon.print(tostring(game.tick_paused)..':'..tostring(game.tick))")
            time.sleep(2)
            report["pause_probe_after"] = kubectl(namespace, "exec", pod, "--", "rcon", "/c rcon.print(tostring(game.tick_paused)..':'..tostring(game.tick))")
            if lifecycle:
                run_lifecycle(namespace, pod, report)
            elif observation:
                run_observation(namespace, pod, report, args)
            elif walk:
                run_walk_stop(namespace, pod, report)
            else:
                pod = run_mine(namespace, pod, report)
            report["result"] = "passed"
            return 0
        finally:
            if pod:
                collect_logs(namespace, pod, report_dir)
            report_dir.mkdir(parents=True, exist_ok=True)
            (report_dir / "report.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            subprocess.run(["kubectl", "delete", "namespace", namespace, "--wait=true", f"--timeout={TIMEOUT_SECONDS}s"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, check=False, timeout=TIMEOUT_SECONDS + 10)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, subprocess.TimeoutExpired, json.JSONDecodeError) as error:
        print(f"fixture runner failed: {error}", file=sys.stderr)
        raise SystemExit(1)
