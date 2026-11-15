# Flag rehearsal

`TARE_REHEARSAL_DEFECT=dispatch-log` is an explicit, local rehearsal defect. It is off when the variable is absent; other values are rejected. `tare build` prints `REHEARSAL DEFECT ENABLED` and records the switch in `target/tare/build.json`. `tare attest` rejects an artifact carrying that marker.

The defect changes one condition only: a transaction whose instruction bytes are exactly ASCII `TAREBAD!` emits `TARE REHEARSAL DEFECT: dispatch-log` before normal dispatch. Stock Anchor still returns its ordinary unknown-instruction error. The extra log is an observable, deliberately introduced divergence; the harness does not filter it.

`new-idl-input.json` is a real signed input targeting the pinned Anchor 0.32.3 new_idl program. It includes exact account snapshots with `rentEpoch` equal to the maximum u64 as decimal strings. The deterministic payer key is only a disposable SVM fixture; no on-chain funds or signature are needed to replay it. The wallet supplied to `tare flag` separately pays for the on-chain Flag account.

First build and attest with the switch **unset**. Then use the returned record address:

```powershell
$env:TARE_REHEARSAL_DEFECT = 'dispatch-log'
tare flag RECORD --host --input C:/path/to/backend/bench/rehearsal/new-idl-input.json --url http://127.0.0.1:7840 --api-url http://127.0.0.1:7810 --keypair C:/path/to/local-wallet.json
Remove-Item Env:TARE_REHEARSAL_DEFECT
```

`flag` verifies the normal record and rebuilds its ordinary artifacts first. It then builds the named defect, executes the input against stock and defective artifacts, uploads the genuine comparison and sends `flag`. Without the switch the same input must be refused with `input does not diverge`; no upload or chain transaction occurs.

The native evidence files in this folder are measurements from the actual ELFs. Validator/API end-to-end transcripts are produced by the lead's run.

Native end-to-end verification completed against the local LiteSVM RPC adapter: normal attestation created record `3A1EaPTVQk66Wb91trSVDu6fcAcr2936JHw3aj49wXe7`; `check` rebuilt in a different directory and printed `matches` after 1,712 inputs; the enabled rehearsal created flag `EANsCfuFntFRaptZtVeELBqkXSBPcfnNpnkYshXiq5kh`. Both accounts were read back from the SVM. This is test-adapter evidence, not a validator or mainnet submission.

The final 0.4.0 native repetition uses record `74LeGmkfeFisd6wgypGNxb39Kprjf2hawU7z8p3wTVsn` and flag `GXRf75kqN36nzzTButzakeCv8MojpVUPUVSijd3CSGgd`. Cross-folder check replays 1,880 generated inputs and prints `matches`; the ordinary flag invocation refuses the same input. `chain-verification-0.4.json` retains the read-back accounts. The RPC adapter is stopped after the check. These addresses exist only in that captured native SVM.
