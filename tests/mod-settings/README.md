# Disposable mod-settings fixture

Use this only with a copied disposable save. It is the documented equivalent of
Factorio's startup mod-settings configuration for lifecycle integration runs:

```ini
[mod-setting-name]
factorio-agent-bridge-actor-id=alfred
```

The setting is intentionally a startup setting. `alfred` is the only default;
operators must choose any other actor ID before creating the test save. Once the
bridge has initialized a save, changing this value is rejected rather than
retargeting a character. The actor starts with the bridge policy `empty` and no
items are granted.

A real headless fixture must set the same value through Factorio's mod-settings
file or command-line fixture mechanism and provide a disposable save path to the
ignored Rust lifecycle test.
