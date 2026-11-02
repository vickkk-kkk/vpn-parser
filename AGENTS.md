# Project guidance

- Keep the desktop egui application and upstream serialized formats compatible while extracting the shared Rust business layer.
- Prefer pure, explicitly parameterized core APIs. Keep network fetching, process execution, system proxy/DNS changes, native VPN lifecycle and filesystem path selection in adapters.
- Android and iOS integration must report unsupported/not-implemented states honestly until native VPN and engine integration exists.
- Preserve upstream license and attribution. Do not include credentials, real subscriptions, private notes, build caches or generated binaries in source changes.
- Coordinate Cargo.toml/Cargo.lock through one integrator. Read docs/client-architecture.md for extraction seams and validation stages.
- Application startup and packaging scripts are side-effectful; they are not substitutes for pure unit tests. Review runtime/download/build-script behavior before selecting a validation command.
