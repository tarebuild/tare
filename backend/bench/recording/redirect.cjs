// Test-process transport hook: redirect only the local validator's HTTP RPC.
const http=require('node:http');
const original=http.request;
const destination=new URL(process.env.TARE_RECORD_PROXY);
http.request=function(input,...args){
 if(typeof input==='string'||input instanceof URL){const url=new URL(input);if(['127.0.0.1','localhost','[::1]'].includes(url.hostname)&&url.port==='8899'){url.hostname=destination.hostname;url.port=destination.port}return original.call(this,url,...args)}
 if(input&&['127.0.0.1','localhost','::1'].includes(input.hostname||input.host)&&Number(input.port)===8899){input={...input,hostname:destination.hostname,host:destination.hostname,port:destination.port}}
 return original.call(this,input,...args);
};

if(globalThis.fetch){const originalFetch=globalThis.fetch;globalThis.fetch=function(input,options){if(typeof input==='string'||input instanceof URL){const url=new URL(input);if(['127.0.0.1','localhost','[::1]'].includes(url.hostname)&&url.port==='8899'){url.hostname=destination.hostname;url.port=destination.port}input=url}return originalFetch.call(this,input,options)}}
