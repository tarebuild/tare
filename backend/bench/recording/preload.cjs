// Runs before the user's test command, after Anchor starts its owned validator.
// Refuses a missing/wrong program and records exact loader/feature diagnostics.
if(process.env.TARE_VALIDATOR_PREFLIGHT){
 const{spawnSync}=require('node:child_process');
 const run=spawnSync(process.execPath,[process.env.TARE_VALIDATOR_PREFLIGHT,process.env.TARE_VALIDATOR_PREFLIGHT_CONFIG],{stdio:'inherit',env:{...process.env,NODE_OPTIONS:'',TARE_VALIDATOR_PREFLIGHT:''}});
 if(run.status!==0)process.exit(run.status||1);
}
