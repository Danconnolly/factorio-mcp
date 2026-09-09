# Optional Kubernetes integration fixture

## Boundary

Kubernetes is an optional provider for the disposable Factorio integration
fixture. It is never a dependency of the Rust unit, contract, static-bridge, or
MCP tests.

The default verification command is deliberately cluster-free:

```text
cargo test --workspace
```

It compiles the two external-fixture entry points but does not run them because
they are marked `#[ignore]`. It must not read kubeconfig, invoke `kubectl`,
start a container, connect to RCON, require a Factorio installation, or mutate a
save.

The two integration tests are explicit opt-in release checks:

```text
cargo test -p fair-play-testkit actor_lifecycle_end_to_end -- --ignored
cargo test -p fair-play-testkit observation_policy_end_to_end -- --ignored
```

They require an operator-provided runner through `FACTORIO_LIFECYCLE_FIXTURE`
or `FACTORIO_OBSERVATION_FIXTURE`, respectively, plus
`FACTORIO_DISPOSABLE_SAVE`. A runner may use Kubernetes, a local headless
server, or another isolated provider that satisfies the same contract.

## Kubernetes provider requirements

A Kubernetes runner must create a unique, disposable namespace for each run and
delete it in a cleanup path regardless of success or failure. It must provision:

- a copied disposable save and test-only bridge-mod configuration;
- a non-logged Kubernetes Secret for the RCON password;
- a private ClusterIP Service for the RCON port;
- persistent per-run storage sufficient to verify save/reload and server restart;
- explicit CPU and memory requests/limits;
- bounded readiness, execution, log-collection, and cleanup timeouts.

A run must retain its server logs, bridge/audit output, contract JSON, save
checksums, assigned Pod/node metadata, and test result as its evidence bundle.
It must never use a production namespace, a valued save, or credentials from a
production deployment.

## Scheduling

The fixture intentionally relies on the cluster's default scheduler. Its Pod
specification must not set `nodeName`, `nodeSelector`, affinity,
`topologySpreadConstraints`, tolerations, a custom `schedulerName`, or other
placement override. Resource requests and limits are required for observability
and containment; placement remains a normal cluster resilience concern.

## Scope of automated evidence

The Kubernetes runner can automate the zero-client lifecycle and bounded
observation checks. It cannot replace the manual real-client differential gate:
the Phase-0 procedure still requires a human-controlled client connected to the
same disposable server for comparison and human-isolation evidence.
