# Deploying agentstow.dev

## Current state

Deployed as the Worker `agentstow-site` (Workers Static Assets). The apex domain and its
certificate are attached by the `custom_domain` route in `wrangler.toml`; no DNS record is
created by hand.

- [x] First deploy — live at <https://agentstow.dev>
- [x] `www.agentstow.dev` → 301 to the apex
- [x] `agentstow.com`, `agentstow.org` (and their `www`) → 301 to the apex
- [x] Auto-deploy on push to `main` — Workers Builds (see below)

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

The API token on this machine **does** have DNS and Ruleset write scope, unlike the one
openroutine.dev was set up with, so all of this was scripted. What was created:

| Zone | DNS | Redirect rule |
| :-- | :-- | :-- |
| `agentstow.dev` | `www` AAAA `100::` proxied (apex is the Worker's own record) | `www.agentstow.dev` |
| `agentstow.com` | apex + `www`, AAAA `100::` proxied | `agentstow.com`, `www.agentstow.com` |
| `agentstow.org` | apex + `www`, AAAA `100::` proxied | `agentstow.org`, `www.agentstow.org` |

Each rule is a single `http_request_dynamic_redirect` entry, 301, target
`concat("https://agentstow.dev", http.request.uri.path)` with `preserve_query_string`.

> **Expect 522s for the first minute.** A newly added hostname answers with 522 until its
> rule and certificate finish propagating — the request reaches the edge and is proxied to
> the `100::` discard address before the redirect rule is live. It resolves itself; two of
> the five hosts did exactly this and then returned 301 without any change. Do not
> "fix" it.

## Auto-deploy

Deploys are automatic: Cloudflare's Workers Builds is connected to `agentstow/agentstow`, and
a push to `main` that touches `site/` deploys the site. A push touching nothing under `site/`
is skipped before a build is queued, and non-production branches get preview versions via
`npx wrangler versions upload` without promoting them.

Connected under **Workers & Pages → `agentstow-site` → Settings → Build** — dashboard only;
there is no CLI or API path (see the history note below). The configuration of record:

| Setting | Value |
| :-- | :-- |
| Git repository | `agentstow/agentstow` |
| Root directory | `site` |
| Build command | *(none)* |
| Deploy command | `npx wrangler deploy` |
| Version command | `npx wrangler versions upload` |
| Production branch | `main` |
| Builds for non-production branches | **on** |
| Build watch paths → include | `site/*` |

The GitHub App is installed on the `agentstow` org, scoped to this one repository. Cloudflare
authenticates builds with its own auto-minted API token (`Workers Builds - <timestamp>`,
visible under Settings → Build); no repository secret is involved. The Worker name in the
dashboard must stay `agentstow-site` — it has to match `name` in `wrangler.toml` or the build
fails.

### History: this replaced a GitHub Actions deploy

Until 2026-08-15 the deploy lived in `.github/workflows/deploy-site.yml`, authenticated by a
scoped `CF_DEPLOY_TOKEN` repository secret — deliberately: the 2026-08-13 survey (docs,
`wrangler`, the `cf` CLI, and the REST API, each checked separately) found no scriptable way
to connect a repository to Workers Builds, and in-repo configuration was judged worth the
token upkeep. Those findings still hold; the connection above was made by hand in the
dashboard. The decision was reversed to converge with `soulmachine/openroutine`, which
deploys the same way. The workflow was deleted, the repo secret removed, the Cloudflare-side
token revoked, and the workflow's post-deploy smoke checks moved into
[Verifying](#verifying) below.

## Verifying

```sh
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev        # 200
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev/docs   # 200
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev/zh     # 200
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev/zh/docs # 200
curl -s -o /dev/null -w '%{http_code}\n' https://agentstow.dev/nope   # 404, served not thrown
curl -s https://agentstow.dev/zh/nope | grep -q 'lang="zh'            # the *Chinese* 404
curl -sI https://www.agentstow.dev        | grep -i location
curl -sI https://agentstow.com            | grep -i location
curl -sI https://agentstow.dev | grep -i content-security-policy      # script-src 'none'
```

Prefer GET (`-o /dev/null -w '%{http_code}'`) over HEAD (`-I`) for status checks — the edge
answers HEAD inconsistently on freshly deployed assets. Expect a minute of intermittent
errors right after a deploy while the new version propagates; sample a dozen requests before
concluding anything is wrong.
