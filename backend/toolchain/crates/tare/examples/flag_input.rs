use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as B64,Engine};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_instruction::Instruction;
use solana_transaction::Transaction;
fn main()->Result<()>{
 let args:Vec<String>=std::env::args().skip(1).collect();
 let key=Keypair::try_from(serde_json::from_slice::<Vec<u8>>(&std::fs::read(&args[0])?)?.as_slice()).map_err(|e|anyhow::anyhow!("key: {e}"))?;
 let idl:serde_json::Value=serde_json::from_slice(&std::fs::read(&args[1])?)?;
 let program=idl["address"].as_str().unwrap().parse()?;
 let data=idl["instructions"][0]["discriminator"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as u8).collect();
 let tx=Transaction::new_signed_with_payer(&[Instruction{program_id:program,accounts:vec![],data}],Some(&key.pubkey()),&[&key],Default::default());
 let input=serde_json::json!({"kind":"generated","label":"initialize-control","transactionBase64":B64.encode(bincode::serialize(&tx)?),"accounts":[{"address":key.pubkey().to_string(),"owner":"11111111111111111111111111111111","lamports":10_000_000_000u64,"executable":false,"dataBase64":""}],"slot":42});
 std::fs::write(&args[2],serde_json::to_vec_pretty(&input)?)?;Ok(())
}