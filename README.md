## ocu

`ocu` is a terminal dashboard for local [OpenCode](https://opencode.ai/) usage.
When OpenCode recorded no cost for a model, `ocu` estimates it from current
[Models.dev](https://models.dev/) pricing and marks it `est.`. It never
modifies OpenCode's database.

Fetched model rates are cached in `$XDG_CACHE_HOME/ocu/models.json` (or
`~/.cache/ocu/models.json`). Pass `--refresh` to update the cache.
