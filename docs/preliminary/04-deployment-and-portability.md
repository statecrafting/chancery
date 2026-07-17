# 04: deployment and portability (PRELIMINARY)

> Captures the confirmed deployment decisions so the library work in
> `00`-`03` is not blocked on them. These are chancery-**service**
> concerns; the three extracted libraries are pure Rust and deployment-
> agnostic (they do not know what Kubernetes is).

## Confirmed decisions

1. **Stay Terraform, portable across clouds (like OAP).** chancery adopts
   OAP's posture: cloud provisioning via Terraform modules that are
   swappable per provider, with cloud-neutral Helm charts and Rauthy on top.
   OAP already ships `azure_core` / `aws_core` / `gcp_core` / `do_core`
   Terraform modules plus `cloud_secrets` / `workload_identity`; the Helm
   charts and Rauthy are cloud-neutral. chancery mirrors this so the cluster
   substrate is a config choice, not a rewrite.
2. **Live target: Hetzner K3s + Flux + Helm-via-CI.** The current OAP
   production cluster is Hetzner K3s (provisioned via the `hetzner-k3s`
   CLI), with Flux v2 GitOps reconciling cluster-level infra (ingress-nginx,
   cert-manager, Rauthy, monitoring, NetworkPolicy/LimitRange baseline) and
   imperative `helm upgrade --install` via CI for the app services. chancery
   deploys the same way. Note the doc-vs-reality drift in OAP
   (`platform/CLAUDE.md` still says Azure AKS); do not inherit that drift,
   document Hetzner-K3s-plus-portable as the real posture.
3. **Platform-level for now, tenant-pivot preserved.** chancery is a shared
   control plane (one deployment governing many founders), like statecraft /
   deployd-api-rs, not a per-tenant app. The open-core / private-config
   boundary is drawn so a later pivot to per-tenant deployment is a
   packaging change, not a rewrite (it would add a `TenantShape` + chart +
   `helm.rs` `include_str!` + factory-encore adapter, per the OAP tenant
   path). Design for platform-level; keep the seams clean enough to pivot.

## Portability model (mirrors OAP `platform/`)

```
Terraform modules (per cloud, swappable)     Helm charts (cloud-neutral)
  hetzner (live) | azure | aws | gcp | do  ->  conv-api, conv-kernel, rauthy
        provisions the cluster + secrets           deployed onto whichever cluster
                                                    ▲
  Flux GitOps: cluster-level infra (ingress, cert-manager, Rauthy, monitoring, NetworkPolicy)
  Helm-via-CI: the two chancery app services (conv-api, conv-kernel)
```

The two chancery services (from Part 2 of the architecture discussion):
- `conv-api`: Encore.ts modular monolith (statecraft-shaped) with an
  embedded React Router v7 SSR review UI; async/outbox on NSQ (Encore
  self-hosted pubsub, already on the cluster).
- `conv-kernel`: Rust axum service (deployd-api-rs-shaped) hosting the
  `action-gate` + `attest-ledger` + `trust-window` libraries and the
  autonomy ladder; Rauthy JWT auth copied from `deployd auth.rs`.

Adding each to the cluster is the cleanest reuse point in the OAP platform
plane: a chart under `charts/<name>/` + a `values-hetzner.yaml` + a
`cd-<name>.yml` workflow calling the generic `helm-deploy` composite action,
plus (optionally) a Flux `HelmRelease` if the service should be
GitOps-managed rather than CI-deployed.

## Why the libraries are unaffected

The extracted libraries are `no-runtime` pure Rust (the gate is a pure
function; the ledger produces/verifies records; the scorer is in-memory).
They compile to `lib` and (for the kernel) `cdylib`/`wasm32` just as OAP's
`policy-kernel` does today. Where `conv-kernel` runs (separate axum service
vs. wasm-in-Node) is a chancery-service decision, recorded here for later,
and does not change any library API:

- **Recommended: separate axum service** (`conv-kernel`), the proven
  deployd-api-rs pattern. A message send is not latency-critical (the ESP
  call dominates), so the network hop is free, and the signed ledger stays
  in one hardened Rust boundary.
- The **outbox is what makes the gate unbypassable**, not in-process-ness:
  route every send through the outbox, have the outbox call `conv-kernel`,
  and the veto holds by construction.

## Not in scope for the library phase

Charts, Terraform, Flux manifests, and the two services are chancery-repo
work that comes **after** the libraries exist (Phase C0 onward in `00`).
This doc exists so those decisions are settled and do not leak into the
library designs. Nothing in `00`-`03` depends on the cluster.
