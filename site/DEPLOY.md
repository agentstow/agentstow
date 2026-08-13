# Deploying agentstow.dev

## Current state

Deployed as the Worker `agentstow-site` (Workers Static Assets). The apex domain and its
certificate are attached by the `custom_domain` route in `wrangler.toml`; no DNS record is
created by hand.

- [ ] First deploy — `cd site && npx wrangler deploy`
- [ ] `www.agentstow.dev` → 301 to the apex
- [ ] `agentstow.com`, `agentstow.org` → 301 to the apex
- [ ] Workers Builds — auto-deploy on push to `main`

## Deploying

```sh
cd site
npx wrangler deploy
```

Auth comes from `CLOUDFLARE_API_TOKEN` in `~/.zshenv`; there is no wrangler OAuth config on
this machine. The zone `agentstow.dev` is id `de5a3b0f14b051def606f3779a439641`.

## Redirects

`_redirects` cannot do domain-level redirects, so each of `www.agentstow.dev`,
`agentstow.com` and `agentstow.org` needs two things: a proxied DNS record so Cloudflare
has something to terminate, and a zone Redirect Rule (a `http_request_dynamic_redirect`
ruleset) sending it to `https://agentstow.dev` with a 301.

An `AAAA` record pointing at `100::` (the discard prefix), proxied, is the conventional
target for a redirect-only hostname — that is what Cloudflare's own custom-domain
attachment produces.

> The API token on this machine can *read* DNS records and rulesets on the zone (both
> return 200). Write scope is untested. If a write returns 403, do it in the dashboard and
> record that here rather than fighting the token — this is what happened on
> openroutine.dev.

## Auto-deploy

Workers Builds is dashboard-only; there is no CLI or API path, because it runs through the
GitHub App install flow. Under **Workers & Pages → `agentstow-site` → Settings → Build**:

| Setting | Value |
| :-- | :-- |
| Git repository | `agentstow/agentstow` |
| Root directory | `site` |
| Build command | *(none)* |
| Deploy command | `npx wrangler deploy` |
| Production branch | `main` |
| Build watch paths → include | `site/*` |

There is no build step — `public/` is already the finished site. The root directory is what
makes `wrangler.toml` findable; without it the deploy runs at the repo root and fails.

## Verifying

```sh
curl -sI https://agentstow.dev            | head -1   # HTTP/2 200
curl -sI https://agentstow.dev/docs       | head -1   # HTTP/2 200
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev/nope   # 404
curl -sI https://www.agentstow.dev        | grep -i location
curl -sI https://agentstow.com            | grep -i location
curl -sI https://agentstow.dev | grep -i content-security-policy      # script-src 'none'
```
