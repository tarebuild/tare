# Source-based account ABI observation selector

The isolated helper replaces the current substring scan with syntax-aware observation detection. `account_layout_observation(&Path) -> anyhow::Result<Option<String>>` has the same call signature as the CLI function. Copy `src/lib.rs` as a private CLI module and delegate that call; add `syn = { version = "2", features = ["full", "visit"] }`, `quote = "1"`, and `proc-macro2 = "1"` to the CLI dependencies. The span-locations feature is optional and is not used.

The helper does not decide that a program implements the raw ABI. It permits the existing raw compilation probe when layout observations only concern known independent local payload types; actual Rust SDK boundaries still trigger the existing type-diagnostic SDK retry. Unknown/generic observations, ambiguous duplicate local names, external type imports or globs, account wrappers, TypeId/type-name observations over accounts, and Rc/RefCell operations remain conservative.

Local structs, enums, unions and aliases recurse through their field types. Qualified unknown types are never resolved by matching only their last segment. Local names shadow the built-in allowlist. Observer aliases imported with `use` and observer function values are inspected. Account attribute and macro token streams are checked; comments and string contents do not trigger a decision.

This is an intentionally conservative source detector, not full Rust name resolution or a proof of arbitrary unsafe code. `size_of_val`, pointer macros, unknown transmute arguments and downcasts select SDK. Unsupported expression and macro expansion name-resolution cases must remain under the raw compile probe plus differential tests; no universal ABI-equivalence claim is made.

Checks: 14 targeted tests pass, covering nested payloads, custom-discriminator account attributes, true AccountInfo fields and aliases, generic types, imported function aliases, known-name shadowing, qualified external types, function values, comments and address macros. All 16 selected real files (custom-discriminator/realloc/lamports/native-system/optional and accountloader-realloc across available versions) now permit the raw compilation probe. No SBF build or performance claim is made by this helper.

A post-handoff guard rejects renamed local imports even when their alias has a built-in payload name. The isolated old/new selector comparison covers all 189 currently measured program roots and reports zero changed decisions; `all-measured-comparison.json` records both selector hashes.
