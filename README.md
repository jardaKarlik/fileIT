# FileIT — desktop app

Smart file organiser for Czech financial advisors. Tauri 2 (Rust) backend +
React/TypeScript frontend. `master` holds the latest desktop release.

## Development

```bash
npm install
npm run tauri dev
```

Release build: `npm run tauri build` (installers in `src-tauri/target/release/bundle/`).

OCR uses the [Indra OCR](https://github.com/datalab-to/indra) sidecar
(`pip install indra-ocr`). If `indra-ocr` is not on `PATH`, set
`INDRA_OCR_COMMAND` to the full path of the executable; `INDRA_OCR_MODELS`
points it at pre-downloaded models.

## Branches

- `master` — latest desktop release
- `lee/awesome-curie-ek1wko` — v1.1.0 in progress (OCR on/off switch)
- `website-archive` — old copy of the marketing website that used to be
  merged into this repo. The live website (Lovable project) is
  `jardaKarlik/fileit-cz-98e2c06b`.
