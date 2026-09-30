# secret-ink
A pseudo-document generator to look like "top secret" documents from markdown files.

## Run

```sh
cargo run
```

The desktop studio starts with `profile.toml`, `input.md`, and the configured font and paper files. Use the Browse buttons to choose different files, adjust the page and ink settings, and save changes back to the active TOML profile.

Update Preview renders a proof with its longest side limited to 768 pixels. Render Full Size writes the configured PNG output, and Open Image launches that file in the system's default image viewer.

To render without opening the desktop studio, use the `render` command:

```sh
cargo run -- render --profile profile.toml --input input.md --output output.png
```

All options are optional. By default, it reads `profile.toml` and `input.md` and writes `output.png` relative to the current working directory. Font and paper paths in the profile are resolved relative to the profile file. Run `cargo run -- render --help` to see the available options.

## Example

![Example generated document](image.png)