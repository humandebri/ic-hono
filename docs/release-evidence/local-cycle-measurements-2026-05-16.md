# Local Cycle Measurements 2026-05-16

Environment:

- `icp` local network / PocketIC
- canister: `edge`
- measurement script: `scripts/measure_local_cycles.mjs`
- bundle: `examples/hono-status/dist/app.bundle.js`
- delta: `before_cycles - after_cycles - idle_burn_elapsed`
- USD conversion: `cycles / 1_000_000_000_000 * 1.33`

Scope:

- Results are local deterministic-environment measurements, not mainnet pricing.
- Query methods are excluded because this method measures canister cycle balance debit.
- HTTP Gateway is excluded; HTTP routes are called through `http_request_update`.
- Cold and warm runtime calls are separated because generation changes invalidate the QuickJS runtime cache.

| operation | repeats | median cycles | median USD | avg cycles | min | max | samples |
|---|---:|---:|---:|---:|---:|---:|---|
| abort_bundle_upload_missing | 5 | 9145219 | $0.000012 | 9145256 | 9145079 | 9145433 | 9145433, 9145337, 9145079, 9145219, 9145212 |
| set_env_replace_16b | 5 | 23729077 | $0.000032 | 23509281 | 22563177 | 23796840 | 22563177, 23796840, 23728136, 23729077, 23729177 |
| http_get_health_cold_after_generation_change | 3 | 106426761 | $0.000142 | 106404130 | 106311597 | 106474033 | 106311597, 106426761, 106474033 |
| http_get_health_warm_update | 5 | 32672020 | $0.000043 | 32666936 | 32614123 | 32702526 | 32614123, 32702526, 32656753, 32672020, 32689258 |
| http_get_incidents_update | 5 | 38896393 | $0.000052 | 38880498 | 38824505 | 38914295 | 38824505, 38870551, 38914295, 38896393, 38896750 |
| http_get_page_update | 5 | 158018986 | $0.000210 | 158078434 | 157872178 | 158291977 | 157872178, 157975607, 158018986, 158291977, 158233426 |
| http_get_demo_write_cache | 5 | 47080866 | $0.000063 | 47062587 | 46905416 | 47197410 | 47197410, 46905416, 47081467, 47080866, 47047779 |
| http_post_incident_create | 5 | 51961241 | $0.000069 | 51949519 | 51750689 | 52149180 | 51750689, 51796438, 52090050, 51961241, 52149180 |
| http_post_incident_resolve | 5 | 148102636 | $0.000197 | 148064867 | 147859247 | 148302084 | 148102636, 147946360, 147859247, 148114010, 148302084 |
| upload_bundle_direct_status_33kb | 3 | 100710243 | $0.000134 | 100749628 | 100684206 | 100854436 | 100854436, 100710243, 100684206 |
| http_check_example_replicated_fetch_warm | 3 | 769365170 | $0.001023 | 769359399 | 769325598 | 769387429 | 769325598, 769365170, 769387429 |
| direct_fetch_outcall_non_replicated | 3 | 746783624 | $0.000993 | 746783753 | 746783461 | 746784176 | 746783624, 746783461, 746784176 |
| direct_fetch_outcall_replicated | 3 | 746808084 | $0.000993 | 746832492 | 746806453 | 746882940 | 746882940, 746808084, 746806453 |

Validation:

- Every measured call checked expected `Ok` or HTTP status.
- Idle burn correction used `Idle cycles burned per day` from `icp canister status`.
- Repeated warm samples are tight enough to distinguish cold runtime start from steady-state route cost.
