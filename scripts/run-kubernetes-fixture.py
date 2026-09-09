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
TIMEOUT_SECONDS = int(os.environ.get("FACTORIO_FIXTURE_TIMEOUT_SECONDS", "180"))


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
                archive.write(path, f"factorio-agent-bridge_0.1.0/{path.relative_to(MOD_ROOT).as_posix()}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


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
                                    "command": ["sh", "-ec", "set -eu; mkdir -p /factorio/config /factorio/mods /factorio/saves; cp /source/bridge.zip /factorio/mods/factorio-agent-bridge_0.1.0.zip; cp /source/mod-list.json /factorio/mods/mod-list.json; cp /secret/rconpw /factorio/config/rconpw; chmod 600 /factorio/config/rconpw"],
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


def assert_ok(response: dict[str, Any], name: str) -> dict[str, Any]:
    if response.get("ok") is not True or not isinstance(response.get("result"), dict):
        fail(f"{name} bridge query was rejected: {response.get('error', {})}")
    return response["result"]


def collect_logs(namespace: str, pod: str, report: Path) -> None:
    report.mkdir(parents=True, exist_ok=True)
    for label, args in {"factorio.log": ["logs", pod, "-c", "factorio"], "pod.json": ["get", "pod", pod, "-o", "json"]}.items():
        try:
            (report / label).write_text(kubectl(namespace, *args), encoding="utf-8")
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
    over_limit = fixed_query(namespace, pod, "over_limit")
    report["over_limit_observation"] = over_limit
    if over_limit.get("ok") is not False or over_limit.get("error", {}).get("code") != "OUT_OF_POLICY":
        fail("radius above the declared local bound was not rejected")
    denied = fixed_query(namespace, pod, "uncharted")
    report["uncharted_observation"] = denied
    if denied.get("ok") is not False or denied.get("error", {}).get("code") != "UNCHARTED":
        fail("uncharted observation was not denied before lookup")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--save", required=True, type=Path, help="copied disposable Factorio save")
    parser.add_argument("--mod-settings", type=Path)
    parser.add_argument("--assert-local-radius", type=int)
    parser.add_argument("--assert-max-results", type=int)
    parser.add_argument("--assert-max-payload-bytes", type=int)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if shutil.which("kubectl") is None:
        fail("kubectl is required for the Kubernetes fixture provider")
    if not args.save.is_file():
        fail(f"disposable save does not exist: {args.save}")
    lifecycle = args.mod_settings is not None
    observation = args.assert_local_radius is not None
    if lifecycle == observation:
        fail("runner invocation must be either lifecycle or observation")
    if lifecycle and not args.mod_settings.is_file():
        fail(f"mod-settings fixture does not exist: {args.mod_settings}")
    if observation and (args.assert_max_results is None or args.assert_max_payload_bytes is None):
        fail("observation fixture requires all asserted limits")

    namespace = f"factorio-fixture-{secrets.token_hex(4)}"
    report_dir = ROOT / "test-reports" / namespace
    report: dict[str, Any] = {"namespace": namespace, "input_save_sha256": sha256(args.save), "mode": "lifecycle" if lifecycle else "observation", "image": IMAGE}
    pod: str | None = None
    with tempfile.TemporaryDirectory(prefix="factorio-fixture-") as temporary:
        temporary_path = Path(temporary)
        archive = temporary_path / "bridge.zip"
        mod_list = temporary_path / "mod-list.json"
        package_bridge(archive)
        mod_list.write_text('{"mods":[{"name":"base","enabled":true},{"name":"factorio-agent-bridge","enabled":true}]}\n', encoding="utf-8")
        password = secrets.token_urlsafe(32)
        try:
            kubectl(None, "apply", "-f", "-", input_text=manifest(namespace))
            kubectl(namespace, "wait", "--for=condition=Ready", "pod/save-stager", f"--timeout={TIMEOUT_SECONDS}s")
            kubectl(namespace, "cp", str(args.save), "save-stager:/fixture/saves/fixture.zip")
            kubectl(namespace, "delete", "pod", "save-stager", "--wait=true", f"--timeout={TIMEOUT_SECONDS}s")
            kubectl(namespace, "create", "configmap", "bridge-inputs", f"--from-file=bridge.zip={archive}", f"--from-file=mod-list.json={mod_list}")
            kubectl(namespace, "create", "secret", "generic", "rcon-password", f"--from-literal=rconpw={password}")
            kubectl(None, "apply", "-f", "-", input_text=server_manifest(namespace))
            pod = wait_for_pod(namespace)
            if lifecycle:
                run_lifecycle(namespace, pod, report)
            else:
                run_observation(namespace, pod, report, args)
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
