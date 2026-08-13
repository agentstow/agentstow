# agentstow.dev

The source for <https://agentstow.dev>. It lives here, inside the repo it documents, so a
change to the CLI and the change it implies on the website can be one commit and one
review.

```
wrangler.toml     Cloudflare Workers Static Assets config
og.html           source for public/og.png (not published — outside public/)
public/           everything that gets served
```

## Working on it

Hand-written HTML and one CSS file. There is no build step, no npm dependency, and no
JavaScript on the site at all — the same discipline the product follows, and what lets
`public/_headers` declare `script-src 'none'` honestly.

```sh
npx wrangler dev        # from this directory; serves public/ locally
npx wrangler deploy     # ships it
```

`docs.html` is served at `/docs` — Workers Static Assets does the extensionless mapping,
so link to `/docs`, never `/docs.html`.

## Design

`public/styles.css` is structured after [vercel.com/design.md](https://vercel.com/design.md),
with three departures documented in a comment at the top of the file: a platform font
stack instead of Geist, a warm ramp read as "monochrome", and a palette defined here
rather than inherited, because agentstow has no web UI.

The accent colour is spent on exactly four things — link text, the one primary button, the
focus ring, and the hairline on the mark. Code samples use weight and dimming rather than
colour, so they survive greyscale.

## Regenerating the artwork

`public/og.png` is a screenshot of `og.html`:

```sh
"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
  --headless --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
  --screenshot=public/og.png --window-size=1200,630 "file://$PWD/og.html"
```

The favicons come from an inline SVG of the mark, rasterised with ImageMagick at 32, 180
and 192 px. Both commands are only needed when the artwork or the palette changes.

## Accuracy

Every claim on these pages traces to `CONTEXT.md`, `docs/adr/`, or
`.scratch/agentstow-v1/spec.md` in the repo root. Two things in particular must not drift:

- **Memory sync is not a feature.** It is an explicit non-goal in the spec. The site says
  so out loud; do not let a tagline reintroduce it.
- The capability matrix on the landing page is the one in `spec.md`. If the registry in
  `src/registry.rs` changes, both move together.
