import fs from 'node:fs';
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
let serial=0;
async function rpc(method,params){const response=await fetch(config.url,{signal:AbortSignal.timeout(5000),method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({jsonrpc:'2.0',id:++serial,method,params})});const value=await response.json();if(value.error)throw Error(`${method}: ${JSON.stringify(value.error)}`);return value.result}
function emit(line){console.log(line)}
const version=await rpc('getVersion',[]);emit(`validator version=${JSON.stringify(version)}`);
const features=config.features||[
 {name:'disable_sbpf_v0_execution',address:'TestFeature11111111111111111111111111111111'},
 {name:'reenable_sbpf_v0_execution',address:'TestFeature21111111111111111111111111111111'},
 {name:'disable_sbpf_v0_v1_v2_deployment',address:'B8JJXCy5amZyWG9r7EnUYLwzXSXTxG7GZ1qZ1qggo83g'}
];
const currentSlot=BigInt(await rpc('getSlot',[{commitment:'confirmed'}]));
for(const item of features){
 const feature=typeof item==='string'?{name:item,address:item}:item;
 const value=await rpc('getAccountInfo',[feature.address,{encoding:'base64',commitment:'confirmed'}]);const account=value.value;
 if(!account){emit(`feature ${feature.name} ${feature.address} absent atSlot=${currentSlot}`);continue}
 const data=Buffer.from(account.data[0],'base64');
 if(account.owner!=='Feature111111111111111111111111111111111111'||data.length<1||data[0]>1||(data[0]===1&&data.length<9)){emit(`feature ${feature.name} ${feature.address} unrecognized owner=${account.owner} dataBytes=${data.length} atSlot=${currentSlot}`);continue}
 const activatedAt=data[0]===1?data.readBigUInt64LE(1):null;
 emit(`feature ${feature.name} ${feature.address} owner=${account.owner} activatedAt=${activatedAt===null?'none':activatedAt} active=${activatedAt!==null&&activatedAt<=currentSlot} atSlot=${currentSlot}`);
}


let effectiveSlot=0n;
for(const target of config.targets){
 const value=await rpc('getAccountInfo',[target.address,{encoding:'base64',commitment:'confirmed'}]);const account=value.value;
 if(!account)throw Error(`Validator preload missing: ${target.address}`);
 let bytes=Buffer.from(account.data[0],'base64');emit(`${target.address} owner=${account.owner} executable=${account.executable} dataBytes=${bytes.length}`);if(!account.executable)throw Error(`Validator preload not executable: ${target.address}`);
 if(account.owner==='BPFLoaderUpgradeab1e11111111111111111111111'&&bytes.readUInt32LE(0)===2){const alphabet='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';let n=BigInt('0x'+bytes.subarray(4,36).toString('hex'));let address='';while(n){address=alphabet[Number(n%58n)]+address;n/=58n}for(const b of bytes.subarray(4,36)){if(b)break;address='1'+address}const data=await rpc('getAccountInfo',[address,{encoding:'base64',commitment:'confirmed'}]);if(!data.value)throw Error(`Missing ProgramData: ${address}`);const programData=Buffer.from(data.value.data[0],'base64');const deploymentSlot=programData.readBigUInt64LE(4);effectiveSlot=effectiveSlot>deploymentSlot+1n?effectiveSlot:deploymentSlot+1n;bytes=programData.subarray(45);emit(`ProgramData ${address} deploymentSlot=${deploymentSlot} ELF bytes=${bytes.length}`)}
 if(bytes.subarray(0,4).toString('hex')==='7f454c46')emit(`ELF machine=${bytes.readUInt16LE(18)} flags=${bytes.readUInt32LE(48)}`);
 if(bytes.subarray(0,4).toString('hex')!=='7f454c46')throw Error(`Validator program does not expose an ELF: ${target.address}`);
 const local=fs.readFileSync(target.elf);if((!bytes.subarray(0,local.length).equals(local)||bytes.subarray(local.length).some(x=>x!==0)))throw Error(`Validator preloaded ELF differs: ${target.address}`);
}

// The upgradeable loader exposes a deployment in the next bank, including
// programs supplied in genesis at slot zero. Wait before running any tests.
const deadline=Date.now()+30000;
let observedSlot=BigInt(await rpc('getSlot',[{commitment:'confirmed'}]));
while(observedSlot<effectiveSlot){
 if(Date.now()>=deadline)throw Error(`Program visibility timed out: slot ${observedSlot}, requires ${effectiveSlot}`);
 await new Promise(resolve=>setTimeout(resolve,100));
 observedSlot=BigInt(await rpc('getSlot',[{commitment:'confirmed'}]));
}
emit(`program availability: effectiveSlot=${effectiveSlot} observedSlot=${observedSlot}`);
