Reading prompt from stdin...
OpenAI Codex v0.147.0
--------
workdir: /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
model: gpt-5.6-luna
provider: openai
approval: never
sandbox: read-only
reasoning effort: high
reasoning summaries: none
session id: 019fe601-04f0-7f22-9d94-03ad72188e02
--------
user
WEB RECON task (read-only sandbox, web search enabled). You research; you do NOT edit files or commit. Print a structured report to stdout. Plain English, no invented jargon.

SUBJECT: Orca — the Mac agent-coding workbench app (the one whose workspace revolves around worktrees; features include a Space disk-usage page with a treemap of worktrees, reclaimable-storage cleanup, rich agent session history cards with long first prompt + latest turns + subagents, a browser overlay whose prompt input takes page annotations). Identify the product precisely first (likely orca.computer or similar, 2025-2026 era agent IDE for macOS); if multiple products are named Orca, pick the agentic coding workbench matching those features and say which you picked.

Research via web search: official site/docs/changelog, launch posts, reviews, videos, Reddit/HN/X threads.

COVER, each with concrete mechanics:
1. Overall layout model: how it centers the workspace on the WORKTREE (vs conversation), main surfaces and navigation.
2. Session history: the session card anatomy (title, long first-prompt block, latest turns, subagent list with per-subagent message counts, worktree path, Resume in New Tab / Continue in New Session / View Log actions), grouping and filtering.
3. Browser overlay: how it overlays the whole app from a bottom trigger, annotation flow into the prompt, viewport presets.
4. Space page: treemap of worktrees by size, reclaimable calculation, protection states (Keep: main / Keep: active), delete flow.
5. Resource management: per-workspace process attribution, CPU/RSS display, stop controls.
6. Anything publicly known about architecture (native Swift? Tauri? how it talks to Claude Code/Codex CLIs, ACP support).

OUTPUT: SECTION per topic. For each pattern: what it does, edge behavior, one-line "worth copying: yes/no + why". Label every claim with source URL; where public info is thin, say so explicitly rather than guessing. End with a top-10 list of patterns most worth adopting.

warning: clamping SessionEnd hook timeout to 3s in /Users/blackcolours/.codex/plugins/cache/openai-codex/codex/1.0.6/hooks/hooks.json
2026-08-09T10:10:54.669735Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
2026-08-09T10:10:54.730256Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when AuthRequired(AuthRequiredError { www_authenticate_header: "Bearer error=\"invalid_request\", error_description=\"No access token was provided in this request\", resource_metadata=\"https://api.githubcopilot.com/.well-known/oauth-protected-resource/mcp/\"" })
codex
I’ll identify the exact Orca product first, then triangulate official material with independent discussions/reviews. I’ll separate verified mechanics from thin or missing public evidence and cite every claim with a source URL.
exec
/bin/zsh -lc 'rg -n -i "orca|agent-coding|worktree|Space disk" /Users/blackcolours/.codex/memories/MEMORY.md | head -80' in /Users/blackcolours/dev/work/worktrees/mac-command-bar/tsk-808-assembly-wave
 succeeded in 0ms:
4:applies_to: cwd=/Users/blackcolours/dev/work/mac-command-bar; reuse_rule=recheck the current worktree, PR #14 checkpoint, Notion status, and native-proof capability before execution; this rollout records a partial dirty-worktree attempt, not mergeable implementation authority.
14:- TSK-808, PR-14, 167a30ae, tsk-808-assembly-wave, contrast, nextTokens.css, themeChrome.css, themeRegistry.ts, themes/contrast.ts, contrastTokens.test.mjs, WP12A, R0, Notion, dirty-worktree, ScreenCaptureKit Code=-3801, CodeLens, Peek
24:- PR #14 / commit `167a30ae389a2280501011186f245a08d28c6725` remains the intentional checkpoint. The partial TSK-808 worktree was `tsk-808-assembly-wave`, based on `beebda6c4dad60cd2782374a36f24737cefda605`; it did not replace the primary `main` checkout. [Task 1]
33:- Symptom: partial work becomes broad dirty WIP. Evidence: 19 tracked files changed, 834 insertions/209 deletions, plus untracked probe/report files. Fix: avoid leaving uncommitted scope after a partial lane; assess and preserve the current worktree before any cleanup. [Task 1]
39:applies_to: cwd=/Users/blackcolours/dev/work/rental-management (protected simulation worktree=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution); reuse_rule=reuse the runtime/database-identification and backup-first sequence for this Azure preview, but recheck routing, deployed SHA, container state, migrations, and persistent data before acting.
136:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline; reuse_rule=reuse pipeline mechanisms only after confirming worktree branch `cdx/session-freshpipeline-tsk-792-fresh-extraction-pipeline-07291201`; GitHub MCP availability is session/configuration-dependent and must be verified with a live profile call.
142:- rollout_summaries/2026-08-01T00-16-28-uLkb-tsk_792_pipeline_m43_standards_codex_plugin.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ec45-7bb0-b3c0-7bc6adf143ff.jsonl, updated_at=2026-08-01T05:35:46+00:00, thread_id=019fbaad-ec45-7bb0-b3c0-7bc6adf143ff, M4.3 committed after independent and adversarial hardening)
152:- rollout_summaries/2026-08-01T00-16-28-uLkb-tsk_792_pipeline_m43_standards_codex_plugin.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ec45-7bb0-b3c0-7bc6adf143ff.jsonl, updated_at=2026-08-01T05:35:46+00:00, thread_id=019fbaad-ec45-7bb0-b3c0-7bc6adf143ff, output and conversion commits verified)
162:- rollout_summaries/2026-08-01T00-16-28-uLkb-tsk_792_pipeline_m43_standards_codex_plugin.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ec45-7bb0-b3c0-7bc6adf143ff.jsonl, updated_at=2026-08-01T05:35:46+00:00, thread_id=019fbaad-ec45-7bb0-b3c0-7bc6adf143ff, M4.5 remains planned, not implemented)
172:- rollout_summaries/2026-08-01T08-05-39-lcN6-verify_github_mcp_connection.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/08/01/rollout-2026-08-01T04-05-39-019fbc5b-7943-7822-b256-1aa4f7c9e7fc.jsonl, updated_at=2026-08-01T08:06:20+00:00, thread_id=019fbc5b-7943-7822-b256-1aa4f7c9e7fc, live connector proof)
211:- hooks.json, guard-test-changes.js, test-failure-reminder.js, SessionStart, guard-worktree-root.js, guard-destructive-git.sh, scripts/hooks/pre-commit, pre-commit-no-python-check.sh, auto-push.sh
230:- Copied Claude lifecycle scripts are not automatically valid Codex hooks. Keep Git hooks (`scripts/hooks/commit-msg`, `pre-commit`, `pre-push`) as enforcement; keep behavioral guidance in AGENTS.md/skills; reserve Codex hooks for narrow deterministic guards such as canonical worktree creation and destructive Git commands. [Task 1]
268:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline; reuse_rule=reuse the claim-validation and round-trip guidance only after confirming branch `cdx/session-freshpipeline-tsk-792-fresh-extraction-pipeline-07291201` and working-tree state; do not treat the metadata-claim implementation as security-reviewed
274:- rollout_summaries/2026-08-01T00-16-28-4BXp-tsk_792_m4_composition_claims_and_810_roundtrip.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ece5-70f1-8433-05ab01d3b5f7.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ece5-70f1-8433-05ab01d3b5f7, M4.1/M4.2 committed; code-set lane and continuation details added)
284:- rollout_summaries/2026-08-01T00-16-28-RdMT-tsk_792_extraction_pipeline_m3_production_promotion.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eddd-7d71-9045-85503ae28083.jsonl, updated_at=2026-07-31T03:01:30+00:00, thread_id=019fbaad-eddd-7d71-9045-85503ae28083, M3.1-M3.7 machinery committed; M4-M8 remain)
294:- rollout_summaries/2026-08-01T00-16-28-6lhc-security_review_pipeline_changes_no_findings.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ece7-7b20-a1de-cf12dc4650ee.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ece7-7b20-a1de-cf12dc4650ee, source inspection returned findings []; no build/tests run)
304:- rollout_summaries/2026-08-01T00-16-28-RWSM-interrupted_metadata_security_review.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ece7-7b20-a1de-cef0644a3094.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ece7-7b20-a1de-cef0644a3094, interrupted before findings, tests, or exploit validation)
338:- Symptom: files are missing under `/Users/blackcolours/dev/work/EdiPlatform`. Cause: the active repository is the `session-freshpipeline` worktree. Fix: start at the rollout cwd and do not infer that the sibling checkout is current. [Task 3][Task 4]
343:scope: Precision-first, single-document Walmart 856 candidate claims and exact refusal corrections in the fresh-pipeline worktree.
344:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline; reuse_rule=reuse only for declared source IDs, operator vocabulary, and single-document extraction constraints; return the requested raw JSON shape exactly.
350:- rollout_summaries/2026-08-01T00-16-28-0ysX-walmart_856_candidate_rule_extraction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee4c-7a43-8738-16a3ce19c42b.jsonl, updated_at=2026-07-31T02:24:59+00:00, thread_id=019fbaad-ee4c-7a43-8738-16a3ce19c42b, overinclusive claims)
351:- rollout_summaries/2026-08-01T00-16-28-FwNU-walmart_856_candidate_rule_extraction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee98-7aa0-87d5-1fa1b605a32e.jsonl, updated_at=2026-07-31T01:48:56+00:00, thread_id=019fbaad-ee98-7aa0-87d5-1fa1b605a32e, output uncertain)
352:- rollout_summaries/2026-08-01T00-16-28-U1GR-walmart_856_rule_extraction_overgeneration.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eddb-7f33-9ba0-98b83cafa3e5.jsonl, updated_at=2026-07-31T02:50:04+00:00, thread_id=019fbaad-eddb-7f33-9ba0-98b83cafa3e5, overgeneration failure)
353:- rollout_summaries/2026-08-01T00-16-28-urRs-walmart_856_edi_claim_extraction_review.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eeaa-7b60-a9b4-0765ac2d2f9e.jsonl, updated_at=2026-07-31T01:41:08+00:00, thread_id=019fbaad-eeaa-7b60-a9b4-0765ac2d2f9e, precision review)
363:- rollout_summaries/2026-08-01T00-16-28-GEUp-walmart_856_candidate_source_id_correction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee48-7ee3-9a9f-5fb91b0734d9.jsonl, updated_at=2026-07-31T02:27:15+00:00, thread_id=019fbaad-ee48-7ee3-9a9f-5fb91b0734d9, corrected candidates)
364:- rollout_summaries/2026-08-01T00-16-28-TSYN-walmart_856_claim_id_checker_correction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee80-7d40-9880-9dfa22430f39.jsonl, updated_at=2026-07-31T01:51:20+00:00, thread_id=019fbaad-ee80-7d40-9880-9dfa22430f39, correction needs strict JSON)
365:- rollout_summaries/2026-08-01T00-16-28-XOjU-walmart_856_candidate_source_id_correction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-edda-7813-a285-53eb2ea5e602.jsonl, updated_at=2026-07-31T02:51:19+00:00, thread_id=019fbaad-edda-7813-a285-53eb2ea5e602, interrupted correction)
366:- rollout_summaries/2026-08-01T00-16-28-lW5c-walmart_856_claim_source_id_correction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee7a-7171-9e48-4f5a862f1002.jsonl, updated_at=2026-07-31T01:53:44+00:00, thread_id=019fbaad-ee7a-7171-9e48-4f5a862f1002, correction uncertain)
390:scope: Narrow, source-to-sink security reviews of developer-run EDI pipeline changes in the session-freshpipeline worktree; retain no-findings conclusions only within each reviewed diff.
391:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline; reuse_rule=use the trust-model and path/tooling checks only after confirming branch `cdx/session-freshpipeline-tsk-792-fresh-extraction-pipeline-07291201`; review new sinks and do not generalize interrupted reviews as clean.
397:- rollout_summaries/2026-08-01T00-16-28-NBWW-pdf_conversion_security_review_no_findings.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ec1a-7a10-9d3c-4b4d2a197642.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ec1a-7a10-9d3c-4b4d2a197642, findings []; source/converted validation)
398:- rollout_summaries/2026-08-01T00-16-28-If3A-security_review_pipeline_output_paths.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ec67-7e13-aeaf-5be0ed8c0d8a.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ec67-7e13-aeaf-5be0ed8c0d8a, findings []; default-output deletion review)
399:- rollout_summaries/2026-08-01T00-16-28-UUJD-security_review_composition_round_trip_certification.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ecf6-7ae2-82d6-42259f57656d.jsonl, updated_at=2026-08-01T00:16:28+00:00, thread_id=019fbaad-ecf6-7ae2-82d6-42259f57656d, findings []; local CLI trust boundary)
400:- rollout_summaries/2026-08-01T00-16-28-mHv7-pdf_conversion_security_review.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eed1-7862-a4b7-9cd519dd2855.jsonl, updated_at=2026-07-31T00:07:41+00:00, thread_id=019fbaad-eed1-7862-a4b7-9cd519dd2855, findings [])
401:- rollout_summaries/2026-08-01T00-16-28-lQml-security_review_document_extraction_no_findings.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eec5-7a53-84e9-85163ed7bd01.jsonl, updated_at=2026-07-31T00:32:50+00:00, thread_id=019fbaad-eec5-7a53-84e9-85163ed7bd01, findings [])
411:- rollout_summaries/2026-08-01T00-16-28-kExw-security_review_qualified_code_set_ids.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ed9f-7722-879f-5bc42385de9e.jsonl, updated_at=2026-07-31T04:16:07+00:00, thread_id=019fbaad-ed9f-7722-879f-5bc42385de9e, findings [])
412:- rollout_summaries/2026-08-01T00-16-28-rx16-walmart_856_security_review_no_vulnerabilities.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-edd9-7282-aa73-9f1aaa5fa400.jsonl, updated_at=2026-07-31T02:55:43+00:00, thread_id=019fbaad-edd9-7282-aa73-9f1aaa5fa400, findings [])
413:- rollout_summaries/2026-08-01T00-16-28-P0Fr-security_review_note_ledger_refactor_no_findings.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee13-7c02-81ce-e140a02364c1.jsonl, updated_at=2026-07-31T02:45:48+00:00, thread_id=019fbaad-ee13-7c02-81ce-e140a02364c1, findings [])
414:- rollout_summaries/2026-08-01T00-16-28-c3z4-security_review_edi_pipeline_llm_readers.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee53-7773-88b9-b41fbd87ae9a.jsonl, updated_at=2026-07-31T02:19:20+00:00, thread_id=019fbaad-ee53-7773-88b9-b41fbd87ae9a, findings [])
415:- rollout_summaries/2026-08-01T00-16-28-xlch-security_review_claude_document_extraction.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ee68-7ae2-8f07-953d281b5b15.jsonl, updated_at=2026-07-31T02:00:05+00:00, thread_id=019fbaad-ee68-7ae2-8f07-953d281b5b15, findings [])
416:- rollout_summaries/2026-08-01T00-16-28-uZXO-security_review_match_emission_extraction_changes.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eefd-7860-a285-d50b9a91ce70.jsonl, updated_at=2026-07-30T19:50:16+00:00, thread_id=019fbaad-eefd-7860-a285-d50b9a91ce70, findings [])
426:- rollout_summaries/2026-08-01T00-16-28-HgdP-interrupted_composition_pipeline_security_review.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-ed9b-7352-a59e-c214f738a4c1.jsonl, updated_at=2026-07-31T07:21:54+00:00, thread_id=019fbaad-ed9b-7352-a59e-c214f738a4c1, interrupted before analysis)
427:- rollout_summaries/2026-08-01T00-16-28-RPxM-security_review_fresh_extraction_pipeline.md (cwd=/Users/blackcolours/dev/work/worktrees/EdiPlatform/session-freshpipeline, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/31/rollout-2026-07-31T20-16-28-019fbaad-eeb1-78e1-83b2-ea2c94ba8dd2.jsonl, updated_at=2026-07-31T01:27:28+00:00, thread_id=019fbaad-eeb1-78e1-83b2-ea2c94ba8dd2, no concluded review)
431:- interrupted security review, session-freshpipeline, no findings, worktree cwd, PipelinePaths, ClaimsFile
441:- Symptom: a security review begins with file-not-found. Cause: a parent checkout was read instead of the rollout worktree. Fix: start from the explicit `session-freshpipeline` cwd. [Task 1][Task 3]
540:- TSK-808, TSK-799, TSK-798, Assembly, docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md, 2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md, contrast, legibility, gpt-5.6-sol, Tauri, Dockview, Monaco, Roslyn, U1
550:- SOL-xhigh, gpt-5.6-sol, xhigh, Luna Max, SOL-medium, WP0, WP12A, R0-R6, ACP, handoff, worktree cleanup, Browser proof, handoff verification PASS
558:- when extending this product wave, the user said existing IDE/session infrastructure does not need to be rebuilt -> extend current session, Dockview, PTY, Git/worktree, Monaco/Roslyn, Settings, and orchestration abstractions [Task 1]
562:- The authoritative plan is `docs/superpowers/plans/2026-08-01-tsk-808-native-workbench-product-wave.md`; use section 18 of `docs/superpowers/plans/2026-08-04-assembly-acp-orchestration-and-orca-shell-amendment.md` instead of retired schedules. TSK-808 remained open/High; TSK-799 and TSK-798 are completed dependencies. [Task 1]
564:- Handoff: `/Users/blackcolours/.Codex/handoffs/mac-command-bar/tsk-808-sol-xhigh-orchestrator.md`. Begin with plan/instruction recovery, routing and task/git verification, preservation of planning edits, then WP0 → WP12A → read-only R0; create an integration worktree only after execution authorization. [Task 2]
566:- No product code, implementation worktree, commit, push, PR, or merge was created by this rollout. [Task 2]
572:- Symptom: existing WIP or infrastructure is replaced during execution. Cause: protected boundaries or existing authorities were not loaded first. Fix: do not reset, stash, overwrite, or reopen TSK-799; do not create duplicate runtime, ledger, terminal, Git, worktree, Paneview, Dockview, Browser, resource, or Roslyn authorities. [Task 1][Task 2]
646:- Automated checks passed, but `pnpm tauri:dev:next` against `/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution` failed acceptance: no lens through `26.565s`; later only two `4 references` rows; Peek had no left-side result rows or selected source. [Task 1]
673:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution; reuse_rule=reuse only after revalidating the handoff, shared WIP, frozen clock, device, and ledger state; no YS-295 implementation or database mutation was completed in this rollout
679:- rollout_summaries/2026-07-30T03-55-41-3VAq-tsk754_ys295_reprioritization_and_codelens_followup.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/29/rollout-2026-07-29T23-55-41-019fb129-e751-7721-a9b3-e31ec1d1775e.jsonl, updated_at=2026-08-01T12:50:30+00:00, thread_id=019fb129-e751-7721-a9b3-e31ec1d1775e, YS-295 reprioritized; official state unchanged)
689:- rollout_summaries/2026-07-30T03-55-41-3VAq-tsk754_ys295_reprioritization_and_codelens_followup.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/29/rollout-2026-07-29T23-55-41-019fb129-e751-7721-a9b3-e31ec1d1775e.jsonl, updated_at=2026-08-01T12:50:30+00:00, thread_id=019fb129-e751-7721-a9b3-e31ec1d1775e, append-only repair remains required)
699:- rollout_summaries/2026-07-30T03-55-41-3VAq-tsk754_ys295_reprioritization_and_codelens_followup.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/29/rollout-2026-07-29T23-55-41-019fb129-e751-7721-a9b3-e31ec1d1775e.jsonl, updated_at=2026-08-01T12:50:30+00:00, thread_id=019fb129-e751-7721-a9b3-e31ec1d1775e, unrelated CodeLens follow-up did not advance the official simulation)
715:- The authoritative checkout is the intentionally dirty `tsk-754-year-simulation-execution` worktree at HEAD `8f0b3c5e1d0c0893161fccd85526133ec00f45a1`; recovery found 711 status entries. Latest proven run was `RUN-20270212-06`; `RUN-20270212-07` was stopped after a read-only preflight. The frozen clock remained `2027-02-12T05:00:00Z` America/New_York. Never reset, stash, clean, switch, delete, or create another worktree around this shared WIP. [Task 1]
723:- Symptom: a supplied cwd lacks the continuation handoff. Cause: it is the wrong repository/check-out. Fix: verify repository identity and locate the authoritative worktree before reading task artifacts [Task 1]
730:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution; reuse_rule=reuse for Rental Command mobile auth/device-proof work, but revalidate current auth types, exact source commit, flavor/package, installed APK, signing, and physical-device unlock state before asserting behavior
736:- rollout_summaries/2026-07-28T18-59-43-tOj2-android_biometric_login_and_mobile_tabs_verification.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/28/rollout-2026-07-28T14-59-43-019faa18-db7d-7873-aaaf-deef3463152c.jsonl, updated_at=2026-07-29T11:18:54+00:00, thread_id=019faa18-db7d-7873-aaaf-deef3463152c, TSK-782/TSK-786 delivered; focused tests and review passed)
746:- rollout_summaries/2026-07-28T18-59-43-tOj2-android_biometric_login_and_mobile_tabs_verification.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/28/rollout-2026-07-28T14-59-43-019faa18-db7d-7873-aaaf-deef3463152c.jsonl, updated_at=2026-07-29T11:18:54+00:00, thread_id=019faa18-db7d-7873-aaaf-deef3463152c, restored APK installed in place; final PIN-unlocked tab inspection blocked)
806:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution with machine-level macOS backup state; reuse_rule=reuse the read-only evidence sequence on this Mac for similar Time Machine browsing incidents, but revalidate the current destination, APFS volume identifier, snapshot catalog, privacy permissions, and `backupd` logs before inferring the same cause
812:- rollout_summaries/2026-07-28T03-19-59-v3T5-timemachine_external_history_intact_browser_stuck.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/27/rollout-2026-07-27T23-19-59-019fa6bc-82b2-7282-9c56-9ed431c339eb.jsonl, updated_at=2026-07-28T03:30:54+00:00, thread_id=019fa6bc-82b2-7282-9c56-9ed431c339eb, external history intact; browser recovery remained unresolved)
838:applies_to: cwd=/Users/blackcolours/Documents/Codex/2026-07-26/ome with global policy in `/Users/blackcolours/.codex/AGENTS.md`; reuse_rule=safe for process audits and browser-session cleanup on this machine, but revalidate live PIDs, session names, worktree ownership, and user approval before terminating anything
868:- A daemon is identifiable as `playwright-core/lib/entry/cliDaemon.js <session-name>`; Chrome descendants use profiles matching `playwright_chromiumdev_profile-*`. Map a daemon to its worktree with `lsof -a -p <daemon-pid> -d cwd`, then aggregate the complete tree’s RSS/CPU by session [Task 1]
875:- Symptom: generic Chrome helpers are killed. Cause: ownership was inferred from process name alone. Fix: validate the named daemon, cwd/worktree, and parent/child ownership first; termination remains approval-gated [Task 1]
925:applies_to: cwd=/Users/blackcolours/dev/work/rental-management; reuse_rule=reuse the UX priorities and browser-proof requirements for related Rental Command UI work, but treat the TSK-747 worktree, deployed navigation, and incomplete status as time-specific and revalidate before acting
940:- when the user explicitly requested “2 other sub-agents,” non-overlapping web-app sections, worktrees, and a reviewer -> for broad UI work use explicit limited ownership boundaries and a plan/review gate [Task 1]
945:- TSK-747 is “Rescue the web UI around the Unit Command Center.” Its planned worktree was `/Users/blackcolours/dev/work/worktrees/rental-management/tsk-747-ui-rescue` on `tsk-747-unit-command-center-ui` [Task 1]
974:- PR #478 merged at `fdd4ecab83b1f46b2f76e5a9057e3bf931cd625c`; TSK-753 was closed “Done and verified,” task branches/worktree were removed, and local `main` was synchronized with `origin/main` at that time [Task 1]
984:applies_to: cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution (planner originated in /Users/blackcolours/dev/work/rental-management); reuse_rule=reuse the scan-first acceptance, ledger, and handoff pattern for related simulations, but revalidate the checkout, database, clock, device, services, and unresolved financial state before resuming
1000:- rollout_summaries/2026-07-25T06-49-47-pF8i-rental_command_year_simulation_handoff_and_execution_checkpo.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/25/rollout-2026-07-25T02-49-47-019f9809-8267-7530-97cc-9bbd4ec0116e.jsonl, updated_at=2026-07-28T01:55:36+00:00, thread_id=019f9809-8267-7530-97cc-9bbd4ec0116e, authoritative execution checkpoint)
1010:- rollout_summaries/2026-07-25T06-49-47-pF8i-rental_command_year_simulation_handoff_and_execution_checkpo.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/25/rollout-2026-07-25T02-49-47-019f9809-8267-7530-97cc-9bbd4ec0116e.jsonl, updated_at=2026-07-28T01:55:36+00:00, thread_id=019f9809-8267-7530-97cc-9bbd4ec0116e, handoff committed as bf65e99c and pushed)
1020:- rollout_summaries/2026-07-28T02-43-05-ZpoV-tsk_754_recover_january_29_handoff.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/27/rollout-2026-07-27T22-43-05-019fa69a-b762-7001-b674-6483b9d3cfa5.jsonl, updated_at=2026-08-02T03:28:44+00:00, thread_id=019fa69a-b762-7001-b674-6483b9d3cfa5, January 29 continuation checkpoint; later tasks supersede runtime state)
1030:- rollout_summaries/2026-07-28T15-42-17-10qt-tsk_754_february_2_atomic_cutover_handoff.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/28/rollout-2026-07-28T11-42-17-019fa964-1948-74e3-9dd2-f9d6d282c051.jsonl, updated_at=2026-07-29T12:10:22+00:00, thread_id=019fa964-1948-74e3-9dd2-f9d6d282c051, focused PostgreSQL tests passed but live proof/integration remain incomplete)
1040:- rollout_summaries/2026-07-28T15-42-17-10qt-tsk_754_february_2_atomic_cutover_handoff.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/28/rollout-2026-07-28T11-42-17-019fa964-1948-74e3-9dd2-f9d6d282c051.jsonl, updated_at=2026-07-29T12:10:22+00:00, thread_id=019fa964-1948-74e3-9dd2-f9d6d282c051, V2 foundation verified; domain ports remain incomplete)
1050:- rollout_summaries/2026-07-28T15-42-17-10qt-tsk_754_february_2_atomic_cutover_handoff.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/28/rollout-2026-07-28T11-42-17-019fa964-1948-74e3-9dd2-f9d6d282c051.jsonl, updated_at=2026-07-29T12:10:22+00:00, thread_id=019fa964-1948-74e3-9dd2-f9d6d282c051, latest authoritative checkpoint; `git diff --check` passed)
1060:- rollout_summaries/2026-07-29T12-28-15-cW6u-tsk_754_february_12_run_06_handoff_ys295_ys297_ys298.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/29/rollout-2026-07-29T08-28-15-019fadd8-d079-7f11-8b4d-6869e52cfc5a.jsonl, updated_at=2026-07-30T04:00:01+00:00, thread_id=019fadd8-d079-7f11-8b4d-6869e52cfc5a, current authoritative continuation checkpoint)
1070:- rollout_summaries/2026-07-29T12-28-15-cW6u-tsk_754_february_12_run_06_handoff_ys295_ys297_ys298.md (cwd=/Users/blackcolours/dev/work/worktrees/rental-management/tsk-754-year-simulation-execution, rollout_path=/Users/blackcolours/.codex/sessions/2026/07/29/rollout-2026-07-29T08-28-15-019fadd8-d079-7f11-8b4d-6869e52cfc5a.jsonl, updated_at=2026-07-30T04:00:01+00:00, thread_id=019fadd8-d079-7f11-8b4d-6869e52cfc5a, UI, PostgreSQL, audit, and outbox reconciliation completed)

web search: 
web search: Orca Mac agent coding workbench worktrees Space disk usage treemap ...
web search: 
web search: "Resume in New Tab" Orca ...
web search: 
web search: site:onorca.dev/docs "reclaimable" ...
web search: 
web search: github stablyai/orca "Resume in New Tab" ...
web search: 
web search: 
web search: 
web search: stablyai Orca launch post agent development environment 2025 ...
web search: 
web search: https://github.com/stablyai/orca/search?q=Keep%3A+active&type=code
web search: 
web search: "Keep: active" "stablyai/orca" ...
web search: 
web search: site:onorca.dev/docs architecture Electron TypeScript native ...
web search: 
web search: site:onorca.dev/docs "Agent Session History" ...
web search: 
web search: 
web search: 
web search: 
web search: 
web search: site:github.com/stablyai/orca/releases "Continue in New Session" ...
web search: 
web search: https://api.github.com/repos/stablyai/orca/git/trees/main?recursive=1
web search: 
web search: github stablyai/orca AgentSessionHistory.tsx ...
web search: 
web search: site:onorca.dev/docs "Space page" ...
web search: 
web search: site:github.com/stablyai/orca/releases "Resource Manager" ...
web search: 
web search: "Keep main" Orca worktree ...
web search: 
web search: site:deepwiki.com/stablyai/orca Space resource manager ...
web search: 
web search: site:deepwiki.com/stablyai/orca "Resource Manager" ...
web search: 
web search: https://github.com/stablyai/orca/blob/main/src/renderer/src/components/right-sidebar/index.tsx
web search: 
web search: 'space'
web search: 
web search: https://github.com/stablyai/orca/blob/main/src/renderer/src/App.tsx
web search: 
web search: 'space'
web search: 
web search: https://github.com/stablyai/orca/blob/main/src/renderer/src/components/workspace-space/WorkspaceSpacePage.tsx
web search: 
web search: github stablyai/orca "WorkspaceSpacePage" ...
context compacted
web search: 
web search: site:news.ycombinator.com stablyai Orca agent worktree IDE ...
web search: 
web search: https://www.onorca.dev/docs/agents/session-history
web search: 
web search: site:onorca.dev/docs/agents/session-history Orca Agent Session History
web search: 
web search: site:onorca.dev/docs "Agent Session History" "12 shown"
web search: 
web search: 
web search: 
web search: site:onorca.dev resource manager CPU RSS Orca
web search: 
web search: site:onorca.dev Space treemap worktrees reclaimable Orca ...
codex
The product is Stably AI’s open-source Orca (`onorca.dev` / `stablyai/orca`), the “worktree IDE for AI coding agents.” I did not select the unrelated Orca research, shell, or orchestration projects. Public evidence is strong for worktrees, session history, browser annotations, process controls, and Electron architecture; it is thin for the exact Space treemap labels and bottom-trigger overlay described in the brief.

## Source and product identification

Orca describes itself as a desktop/mobile/VPS environment for running Claude Code, Codex, OpenCode, Pi, and other CLI agents in parallel worktrees. It is open source under MIT. [Official site](https://www.onorca.dev/docs), [GitHub repository](https://github.com/stablyai/orca)

A secondary launch report dates the public launch to July 2026. Reddit discussion independently confirms the worktree, embedded-browser, SSH, mobile, and multi-agent positioning. [Writeble launch report](https://www.writeble.com/2026/07/13/stably-ai-launches-orca-an-agent-development-environment-for-running-fleets-of-parallel-agents/), [Reddit discussion](https://www.reddit.com/r/WebAfterAI/comments/1v017c1/orca_is_trending_for_running_fleets_of_coding/)

I found no indexed Orca-specific Hacker News thread or product video. HN results were other products such as “Orc,” Emdash, and Baton. Orca’s official documentation links to `@orca_build` on X, but I found no indexed thread with useful feature details. [Example unrelated HN result](https://news.ycombinator.com/item?id=47441323), [official X link](https://x.com/orca_build)

## 1. Overall layout: workspace centered on the worktree

**What it does**

- The worktree is the primary unit, not the conversation. Each task gets a real Git worktree with its own branch, files, agent terminal, editor tabs, browser tabs, and terminal panes. [Worktree model](https://www.onorca.dev/docs/model/worktrees)
- The sidebar groups worktrees by project/repository. It has filtering, pinning, project and host scopes, and a `Cmd-J` Jump Palette for searching repositories, worktrees, files, agents, and tabs. [Sidebar and filtering](https://www.onorca.dev/docs/model/worktrees), [Jump Palette](https://www.onorca.dev/docs/model/quick-open)
- The documented lifecycle is Create → Work → Review → Ship → Archive/Delete. Diff review, annotations, commits, pushes, and PR actions stay inside the worktree context. [Worktree lifecycle](https://www.onorca.dev/docs/model/worktrees)
- Orca supports split panes so several worktrees and agents can be watched together. [Three-agent walkthrough](https://www.onorca.dev/docs/first-session)

**Edge behavior**

- Worktree creation runs in the background; failures surface with retry, and users can switch to other worktrees while setup continues. [Worktree creation behavior](https://www.onorca.dev/docs/model/worktrees)
- Externally created Git worktrees can be imported or left hidden. Deleting an Orca worktree removes both its directory and branch, with confirmation. [External worktrees and deletion](https://www.onorca.dev/docs/model/worktrees)
- Remote worktrees run the files and agents remotely while the editor and UI remain local. [SSH worktrees](https://www.onorca.dev/docs/ssh), [run modes](https://www.onorca.dev/docs/ways-to-run)

**Worth copying: yes.** The worktree-first model gives every agent a clear filesystem, branch, review target, and cleanup boundary.

## 2. Session history

**What it does**

- The right sidebar has an “Agent Session History” panel that scans on-disk transcripts from supported CLIs. [Session history](https://www.onorca.dev/docs/agents/session-history)
- The header shows counts such as `12 shown · 47 recent` and supports search over session title, working directory, branch, model, and conversation preview. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Scope options are Workspace, Project, and All. View options include agent selection, Last updated/Created sorting, grouping by Project/Folder/Agent, and hiding empty sessions. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Selecting a row exposes working directory, branch, model, message count, the untruncated first prompt, and latest conversation turns. It supports copying the full initial request. [Session details](https://www.onorca.dev/docs/agents/session-history)
- Resume invokes provider-specific commands such as `claude --resume <id>`, `codex resume <id>`, `pi --session <file>`, and `cursor-agent --resume <id>`. It also exposes copy-resume-command, copy-session-ID, copy-log-path, open-log, reveal-log, and open-cwd actions. [Resume actions](https://www.onorca.dev/docs/agents/session-history)

**What is not publicly verified**

- The current public documentation does not show the exact labels “Resume in New Tab,” “Continue in New Session,” or “View Log.” It documents equivalent actions, and a release note mentions Codex “Continue agent work in a new session.” [Session history](https://www.onorca.dev/docs/agents/session-history), [releases](https://github.com/stablyai/orca/releases)
- I found no public documentation proving that each history card contains a subagent list with per-subagent message counts. The docs expose a session’s total message count and the separate Agents feed, but not that exact card anatomy. [Session history](https://www.onorca.dev/docs/agents/session-history), [Agents feed](https://www.onorca.dev/docs/activity)

**Edge behavior**

- Remote workspaces can browse local history, but resume actions only run locally. [Remote limitation](https://www.onorca.dev/docs/agents/session-history)
- If a transcript or provider session file disappears, Resume may be unavailable; the raw log can still be opened when present. [Resume limitations](https://www.onorca.dev/docs/agents/session-history)
- Agent hibernation resumes supported providers automatically, but falls back to a fresh prompt if the provider has deleted or rotated the session. [Hibernation](https://www.onorca.dev/docs/agents/hibernation)

**Worth copying: yes.** Searchable history plus the original prompt, recent turns, working directory, and provider-specific resume command solves the “which agent was doing what?” problem.

## 3. Browser overlay and annotations

**What it does**

- Orca embeds a Chromium browser per worktree. Design Mode turns the browser into a pointer-to-code tool: enable it in the browser toolbar, hover to highlight an element, then click it. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
- The captured attachment includes the element’s HTML, nearby DOM, computed CSS, cropped screenshot, and source file/line when a development source map is available. It is sent to the active agent terminal as one attachment. [Design Mode capture](https://www.onorca.dev/docs/browser/design-mode)
- The intended loop is click → describe change → agent edits source → Orca hot-reloads → click again to verify. [Design Mode loop](https://www.onorca.dev/docs/browser/design-mode)
- The browser supports viewport emulation through Chromium DevTools Protocol and CLI device commands. [CLI reference](https://www.onorca.dev/docs/cli/reference)

**What is not publicly verified**

- Current public docs describe a browser-toolbar toggle, not a bottom trigger that overlays the entire Orca app.
- They describe an attachment entering the active terminal, not a separately documented prompt-overlay composer.
- The docs do not publish a complete list of viewport presets; they document custom viewport/device emulation.

**Edge behavior**

- Source file and line information depends on a source map; without one, the annotation still contains DOM, CSS, and image context. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
- The annotation goes to the active agent terminal, so the selected worktree/agent context matters. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)

**Worth copying: yes.** The DOM + computed-style + screenshot bundle is a compact, high-signal UI bug report. Copy the annotation loop; treat the exact bottom-overlay interaction as unverified.

## 4. Space page and reclaimable storage

**Public evidence is thin.**

- Orca’s renderer source contains first-class `WorkspaceSpacePage` and `WorkspaceCleanupDialog` components, confirming that a Space/cleanup surface exists in the application. [App source](https://github.com/stablyai/orca/blob/main/src/renderer/src/App.tsx)
- Public worktree documentation confirms archive/delete actions, directory-and-branch deletion, and confirmation. [Worktree deletion](https://www.onorca.dev/docs/model/worktrees)
- Release notes mention worktree deletion safety checks, including verifying exited PTYs before blocking deletion. [Releases](https://github.com/stablyai/orca/releases)

I could not find public documentation for:

- a treemap of worktrees by disk size;
- the exact reclaimable-storage formula;
- whether shared Git objects, caches, `node_modules`, or browser data are included;
- the labels “Keep: main” and “Keep: active”;
- the exact multi-step delete flow.

Those details should be treated as observed-product behavior, not independently verified public documentation.

**Edge behavior**

The only documented cleanup guarantees are confirmation, removal of the worktree directory and branch, and process-safety checks. Do not infer the Space calculation from ordinary worktree size: shared directories and Git object storage can change the real reclaimable amount. [Worktree model](https://www.onorca.dev/docs/model/worktrees)

**Worth copying: yes, conceptually.** A size-ranked cleanup view with explicit protection states is valuable, but the calculation and deletion safety rules must be specified before implementation.

## 5. Resource management

**What it does**

- Orca’s status bar has a Memory and CPU popover showing live per-process memory and CPU, with inline controls to stop runaway processes. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
- The release notes also say long daemon sessions checkpoint history to disk instead of retaining the full log in memory. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
- Orca warns that inactive worktrees keep file watchers alive and that many browser tabs are major RAM users. [Troubleshooting](https://www.onorca.dev/docs/troubleshooting)

**What is not publicly verified**

- The changelog confirms per-process attribution, but not the exact UI for grouping processes by workspace.
- It says “memory,” not specifically RSS, and does not document the attribution algorithm for descendants, remote processes, or shared daemons.

**Edge behavior**

Agent hibernation is an experimental option that stops eligible idle terminals and later resumes them; unsupported providers remain running. [Hibernation](https://www.onorca.dev/docs/agents/hibernation)

**Worth copying: yes.** Live CPU/memory visibility and a nearby stop action are practical guardrails for a fleet of terminals and browsers.

## 6. Architecture and agent integration

**What is publicly supported**

- Orca is Electron-based and ships for macOS, Windows, and Linux. [Install documentation](https://www.onorca.dev/docs/install)
- Its repository package manifest identifies Electron, Electron Vite, React/TypeScript tooling, `node-pty`, xterm, Monaco, SSH support, and browser automation dependencies. [package.json](https://github.com/stablyai/orca/blob/main/package.json)
- The supported-agent model is deliberately CLI-first: Orca launches any agent process in a terminal, with deeper integrations for status, hooks, usage, and account switching where available. [Supported agents](https://www.onorca.dev/docs/agents/supported)
- Agent state is detected from terminal OSC titles. Custom CLIs can be registered with a command, startup hook, and status behavior. [Agents and sessions](https://www.onorca.dev/docs/model/agents-sessions), [custom CLI agents](https://www.onorca.dev/docs/agents/custom-cli)
- Resume is also CLI-native: Orca reconstructs each provider’s resume command rather than owning the conversation protocol. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Hooks are written to disk and reloaded so long-lived PTYs continue reaching the live Orca server after an app restart. [Hooks and memory](https://www.onorca.dev/docs/agents/hooks-memory)
- Remote operation uses SSH or a self-hosted Orca server; agents and worktrees can live on another machine. [Ways to run Orca](https://www.onorca.dev/docs/ways-to-run), [SSH worktrees](https://www.onorca.dev/docs/ssh)

**Native Swift/Tauri and ACP**

- I found no evidence that the desktop shell is Swift or Tauri; the official install documentation explicitly calls it Electron-based. [Install documentation](https://www.onorca.dev/docs/install)
- Orca does provide native platform helpers for computer-use operations, including macOS Accessibility and Screen Recording support, but that is separate from the desktop shell. [Computer use](https://www.onorca.dev/docs/cli/computer-use)
- I found no official documentation or repository reference establishing Agent Client Protocol (ACP) support. The documented integration surface is CLI processes, hooks, PTY status, transcript files, SSH, and Orca’s own CLI. [Supported agents](https://www.onorca.dev/docs/agents/supported), [CLI overview](https://www.onorca.dev/docs/cli/overview)
- The optional native Chat UI is explicitly experimental and layered over the existing PTY; the terminal remains the source of truth. [Native Chat](https://www.onorca.dev/docs/agents/native-chat)

**Worth copying: yes for the CLI boundary; no for assuming ACP.** Launching existing provider CLIs preserves their capabilities and avoids rebuilding every provider’s protocol, while hooks and resume commands provide useful integration points.

## Top 10 patterns most worth adopting

1. Make the worktree/project the primary navigation unit. [Worktrees](https://www.onorca.dev/docs/model/worktrees)
2. Give every agent its own branch, files, terminal, and browser context. [Worktrees](https://www.onorca.dev/docs/model/worktrees)
3. Use project-grouped sidebar navigation plus a fast jump/search surface. [Sidebar](https://www.onorca.dev/docs/model/worktrees), [Jump Palette](https://www.onorca.dev/docs/model/quick-open)
4. Build session history around the original prompt, latest turns, cwd, branch, model, and message count. [Session history](https://www.onorca.dev/docs/agents/session-history)
5. Provide provider-specific resume commands, raw-log access, and “open working directory.” [Session history](https://www.onorca.dev/docs/agents/session-history)
6. Let users scope, group, sort, and filter history by workspace, project, folder, and agent. [Session history](https://www.onorca.dev/docs/agents/session-history)
7. Turn browser clicks into structured agent context: DOM, styles, screenshot, and source location. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
8. Add live CPU/memory monitoring with an immediate stop control. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
9. Make cleanup explicit, confirm destructive actions, and validate agent processes before deletion. [Worktrees](https://www.onorca.dev/docs/model/worktrees), [releases](https://github.com/stablyai/orca/releases)
10. Keep the provider boundary CLI-first, then add hooks, status, hibernation, and remote execution incrementally. [Supported agents](https://www.onorca.dev/docs/agents/supported), [hibernation](https://www.onorca.dev/docs/agents/hibernation), [ways to run Orca](https://www.onorca.dev/docs/ways-to-run)
tokens used
266,366
The product is Stably AI’s open-source Orca (`onorca.dev` / `stablyai/orca`), the “worktree IDE for AI coding agents.” I did not select the unrelated Orca research, shell, or orchestration projects. Public evidence is strong for worktrees, session history, browser annotations, process controls, and Electron architecture; it is thin for the exact Space treemap labels and bottom-trigger overlay described in the brief.

## Source and product identification

Orca describes itself as a desktop/mobile/VPS environment for running Claude Code, Codex, OpenCode, Pi, and other CLI agents in parallel worktrees. It is open source under MIT. [Official site](https://www.onorca.dev/docs), [GitHub repository](https://github.com/stablyai/orca)

A secondary launch report dates the public launch to July 2026. Reddit discussion independently confirms the worktree, embedded-browser, SSH, mobile, and multi-agent positioning. [Writeble launch report](https://www.writeble.com/2026/07/13/stably-ai-launches-orca-an-agent-development-environment-for-running-fleets-of-parallel-agents/), [Reddit discussion](https://www.reddit.com/r/WebAfterAI/comments/1v017c1/orca_is_trending_for_running_fleets_of_coding/)

I found no indexed Orca-specific Hacker News thread or product video. HN results were other products such as “Orc,” Emdash, and Baton. Orca’s official documentation links to `@orca_build` on X, but I found no indexed thread with useful feature details. [Example unrelated HN result](https://news.ycombinator.com/item?id=47441323), [official X link](https://x.com/orca_build)

## 1. Overall layout: workspace centered on the worktree

**What it does**

- The worktree is the primary unit, not the conversation. Each task gets a real Git worktree with its own branch, files, agent terminal, editor tabs, browser tabs, and terminal panes. [Worktree model](https://www.onorca.dev/docs/model/worktrees)
- The sidebar groups worktrees by project/repository. It has filtering, pinning, project and host scopes, and a `Cmd-J` Jump Palette for searching repositories, worktrees, files, agents, and tabs. [Sidebar and filtering](https://www.onorca.dev/docs/model/worktrees), [Jump Palette](https://www.onorca.dev/docs/model/quick-open)
- The documented lifecycle is Create → Work → Review → Ship → Archive/Delete. Diff review, annotations, commits, pushes, and PR actions stay inside the worktree context. [Worktree lifecycle](https://www.onorca.dev/docs/model/worktrees)
- Orca supports split panes so several worktrees and agents can be watched together. [Three-agent walkthrough](https://www.onorca.dev/docs/first-session)

**Edge behavior**

- Worktree creation runs in the background; failures surface with retry, and users can switch to other worktrees while setup continues. [Worktree creation behavior](https://www.onorca.dev/docs/model/worktrees)
- Externally created Git worktrees can be imported or left hidden. Deleting an Orca worktree removes both its directory and branch, with confirmation. [External worktrees and deletion](https://www.onorca.dev/docs/model/worktrees)
- Remote worktrees run the files and agents remotely while the editor and UI remain local. [SSH worktrees](https://www.onorca.dev/docs/ssh), [run modes](https://www.onorca.dev/docs/ways-to-run)

**Worth copying: yes.** The worktree-first model gives every agent a clear filesystem, branch, review target, and cleanup boundary.

## 2. Session history

**What it does**

- The right sidebar has an “Agent Session History” panel that scans on-disk transcripts from supported CLIs. [Session history](https://www.onorca.dev/docs/agents/session-history)
- The header shows counts such as `12 shown · 47 recent` and supports search over session title, working directory, branch, model, and conversation preview. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Scope options are Workspace, Project, and All. View options include agent selection, Last updated/Created sorting, grouping by Project/Folder/Agent, and hiding empty sessions. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Selecting a row exposes working directory, branch, model, message count, the untruncated first prompt, and latest conversation turns. It supports copying the full initial request. [Session details](https://www.onorca.dev/docs/agents/session-history)
- Resume invokes provider-specific commands such as `claude --resume <id>`, `codex resume <id>`, `pi --session <file>`, and `cursor-agent --resume <id>`. It also exposes copy-resume-command, copy-session-ID, copy-log-path, open-log, reveal-log, and open-cwd actions. [Resume actions](https://www.onorca.dev/docs/agents/session-history)

**What is not publicly verified**

- The current public documentation does not show the exact labels “Resume in New Tab,” “Continue in New Session,” or “View Log.” It documents equivalent actions, and a release note mentions Codex “Continue agent work in a new session.” [Session history](https://www.onorca.dev/docs/agents/session-history), [releases](https://github.com/stablyai/orca/releases)
- I found no public documentation proving that each history card contains a subagent list with per-subagent message counts. The docs expose a session’s total message count and the separate Agents feed, but not that exact card anatomy. [Session history](https://www.onorca.dev/docs/agents/session-history), [Agents feed](https://www.onorca.dev/docs/activity)

**Edge behavior**

- Remote workspaces can browse local history, but resume actions only run locally. [Remote limitation](https://www.onorca.dev/docs/agents/session-history)
- If a transcript or provider session file disappears, Resume may be unavailable; the raw log can still be opened when present. [Resume limitations](https://www.onorca.dev/docs/agents/session-history)
- Agent hibernation resumes supported providers automatically, but falls back to a fresh prompt if the provider has deleted or rotated the session. [Hibernation](https://www.onorca.dev/docs/agents/hibernation)

**Worth copying: yes.** Searchable history plus the original prompt, recent turns, working directory, and provider-specific resume command solves the “which agent was doing what?” problem.

## 3. Browser overlay and annotations

**What it does**

- Orca embeds a Chromium browser per worktree. Design Mode turns the browser into a pointer-to-code tool: enable it in the browser toolbar, hover to highlight an element, then click it. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
- The captured attachment includes the element’s HTML, nearby DOM, computed CSS, cropped screenshot, and source file/line when a development source map is available. It is sent to the active agent terminal as one attachment. [Design Mode capture](https://www.onorca.dev/docs/browser/design-mode)
- The intended loop is click → describe change → agent edits source → Orca hot-reloads → click again to verify. [Design Mode loop](https://www.onorca.dev/docs/browser/design-mode)
- The browser supports viewport emulation through Chromium DevTools Protocol and CLI device commands. [CLI reference](https://www.onorca.dev/docs/cli/reference)

**What is not publicly verified**

- Current public docs describe a browser-toolbar toggle, not a bottom trigger that overlays the entire Orca app.
- They describe an attachment entering the active terminal, not a separately documented prompt-overlay composer.
- The docs do not publish a complete list of viewport presets; they document custom viewport/device emulation.

**Edge behavior**

- Source file and line information depends on a source map; without one, the annotation still contains DOM, CSS, and image context. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
- The annotation goes to the active agent terminal, so the selected worktree/agent context matters. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)

**Worth copying: yes.** The DOM + computed-style + screenshot bundle is a compact, high-signal UI bug report. Copy the annotation loop; treat the exact bottom-overlay interaction as unverified.

## 4. Space page and reclaimable storage

**Public evidence is thin.**

- Orca’s renderer source contains first-class `WorkspaceSpacePage` and `WorkspaceCleanupDialog` components, confirming that a Space/cleanup surface exists in the application. [App source](https://github.com/stablyai/orca/blob/main/src/renderer/src/App.tsx)
- Public worktree documentation confirms archive/delete actions, directory-and-branch deletion, and confirmation. [Worktree deletion](https://www.onorca.dev/docs/model/worktrees)
- Release notes mention worktree deletion safety checks, including verifying exited PTYs before blocking deletion. [Releases](https://github.com/stablyai/orca/releases)

I could not find public documentation for:

- a treemap of worktrees by disk size;
- the exact reclaimable-storage formula;
- whether shared Git objects, caches, `node_modules`, or browser data are included;
- the labels “Keep: main” and “Keep: active”;
- the exact multi-step delete flow.

Those details should be treated as observed-product behavior, not independently verified public documentation.

**Edge behavior**

The only documented cleanup guarantees are confirmation, removal of the worktree directory and branch, and process-safety checks. Do not infer the Space calculation from ordinary worktree size: shared directories and Git object storage can change the real reclaimable amount. [Worktree model](https://www.onorca.dev/docs/model/worktrees)

**Worth copying: yes, conceptually.** A size-ranked cleanup view with explicit protection states is valuable, but the calculation and deletion safety rules must be specified before implementation.

## 5. Resource management

**What it does**

- Orca’s status bar has a Memory and CPU popover showing live per-process memory and CPU, with inline controls to stop runaway processes. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
- The release notes also say long daemon sessions checkpoint history to disk instead of retaining the full log in memory. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
- Orca warns that inactive worktrees keep file watchers alive and that many browser tabs are major RAM users. [Troubleshooting](https://www.onorca.dev/docs/troubleshooting)

**What is not publicly verified**

- The changelog confirms per-process attribution, but not the exact UI for grouping processes by workspace.
- It says “memory,” not specifically RSS, and does not document the attribution algorithm for descendants, remote processes, or shared daemons.

**Edge behavior**

Agent hibernation is an experimental option that stops eligible idle terminals and later resumes them; unsupported providers remain running. [Hibernation](https://www.onorca.dev/docs/agents/hibernation)

**Worth copying: yes.** Live CPU/memory visibility and a nearby stop action are practical guardrails for a fleet of terminals and browsers.

## 6. Architecture and agent integration

**What is publicly supported**

- Orca is Electron-based and ships for macOS, Windows, and Linux. [Install documentation](https://www.onorca.dev/docs/install)
- Its repository package manifest identifies Electron, Electron Vite, React/TypeScript tooling, `node-pty`, xterm, Monaco, SSH support, and browser automation dependencies. [package.json](https://github.com/stablyai/orca/blob/main/package.json)
- The supported-agent model is deliberately CLI-first: Orca launches any agent process in a terminal, with deeper integrations for status, hooks, usage, and account switching where available. [Supported agents](https://www.onorca.dev/docs/agents/supported)
- Agent state is detected from terminal OSC titles. Custom CLIs can be registered with a command, startup hook, and status behavior. [Agents and sessions](https://www.onorca.dev/docs/model/agents-sessions), [custom CLI agents](https://www.onorca.dev/docs/agents/custom-cli)
- Resume is also CLI-native: Orca reconstructs each provider’s resume command rather than owning the conversation protocol. [Session history](https://www.onorca.dev/docs/agents/session-history)
- Hooks are written to disk and reloaded so long-lived PTYs continue reaching the live Orca server after an app restart. [Hooks and memory](https://www.onorca.dev/docs/agents/hooks-memory)
- Remote operation uses SSH or a self-hosted Orca server; agents and worktrees can live on another machine. [Ways to run Orca](https://www.onorca.dev/docs/ways-to-run), [SSH worktrees](https://www.onorca.dev/docs/ssh)

**Native Swift/Tauri and ACP**

- I found no evidence that the desktop shell is Swift or Tauri; the official install documentation explicitly calls it Electron-based. [Install documentation](https://www.onorca.dev/docs/install)
- Orca does provide native platform helpers for computer-use operations, including macOS Accessibility and Screen Recording support, but that is separate from the desktop shell. [Computer use](https://www.onorca.dev/docs/cli/computer-use)
- I found no official documentation or repository reference establishing Agent Client Protocol (ACP) support. The documented integration surface is CLI processes, hooks, PTY status, transcript files, SSH, and Orca’s own CLI. [Supported agents](https://www.onorca.dev/docs/agents/supported), [CLI overview](https://www.onorca.dev/docs/cli/overview)
- The optional native Chat UI is explicitly experimental and layered over the existing PTY; the terminal remains the source of truth. [Native Chat](https://www.onorca.dev/docs/agents/native-chat)

**Worth copying: yes for the CLI boundary; no for assuming ACP.** Launching existing provider CLIs preserves their capabilities and avoids rebuilding every provider’s protocol, while hooks and resume commands provide useful integration points.

## Top 10 patterns most worth adopting

1. Make the worktree/project the primary navigation unit. [Worktrees](https://www.onorca.dev/docs/model/worktrees)
2. Give every agent its own branch, files, terminal, and browser context. [Worktrees](https://www.onorca.dev/docs/model/worktrees)
3. Use project-grouped sidebar navigation plus a fast jump/search surface. [Sidebar](https://www.onorca.dev/docs/model/worktrees), [Jump Palette](https://www.onorca.dev/docs/model/quick-open)
4. Build session history around the original prompt, latest turns, cwd, branch, model, and message count. [Session history](https://www.onorca.dev/docs/agents/session-history)
5. Provide provider-specific resume commands, raw-log access, and “open working directory.” [Session history](https://www.onorca.dev/docs/agents/session-history)
6. Let users scope, group, sort, and filter history by workspace, project, folder, and agent. [Session history](https://www.onorca.dev/docs/agents/session-history)
7. Turn browser clicks into structured agent context: DOM, styles, screenshot, and source location. [Design Mode](https://www.onorca.dev/docs/browser/design-mode)
8. Add live CPU/memory monitoring with an immediate stop control. [Changelog v1.3.23](https://www.onorca.dev/changelog/1-3-23)
9. Make cleanup explicit, confirm destructive actions, and validate agent processes before deletion. [Worktrees](https://www.onorca.dev/docs/model/worktrees), [releases](https://github.com/stablyai/orca/releases)
10. Keep the provider boundary CLI-first, then add hooks, status, hibernation, and remote execution incrementally. [Supported agents](https://www.onorca.dev/docs/agents/supported), [hibernation](https://www.onorca.dev/docs/agents/hibernation), [ways to run Orca](https://www.onorca.dev/docs/ways-to-run)
