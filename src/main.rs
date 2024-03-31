use std::{ env, path::Path };
use c_kzg::{
    KzgCommitment, 
    KzgProof, 
    KzgSettings,
    Blob
};
use alloy::{
    network::{ Ethereum, TxSigner },
    providers::{ Provider, ProviderBuilder },
    rpc::client::RpcClient,
    signers::{
        wallet::LocalWallet,
    },
    primitives::{ bytes, U256, FixedBytes},
    consensus::{
        TxEip4844,
        TxEip4844Variant,
        BlobTransactionSidecar,
        SignableTransaction,
        TxEnvelope,
    },
    eips::{
        eip2930::AccessList,
        eip4844::BYTES_PER_BLOB,
        eip2718::Encodable2718
    }
};

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let wallet: LocalWallet = env!("PK").parse()?;
    let addr = wallet.address();
    let client = RpcClient::new_http(env!("RPC_URL").parse()?);
    let provider = ProviderBuilder::<_,Ethereum>::new()
                    .on_client(client);

    let nonce = provider.get_transaction_count(addr, None).await?;
    let estimation = provider.estimate_eip1559_fees(None).await?;

    let trusted_setup = KzgSettings::load_trusted_setup_file(Path::new("./trusted_setup.txt"))?;
    let blob_data = [41; BYTES_PER_BLOB];
    let blob = Blob::new(blob_data);
    let commitment = KzgCommitment::blob_to_kzg_commitment(&blob, &trusted_setup)?;
    let proof = KzgProof::compute_blob_kzg_proof(&blob, &commitment.to_bytes(), &trusted_setup)?;

    let sidecar = BlobTransactionSidecar::new(
        vec![FixedBytes::new(blob_data)],
        vec![FixedBytes::new(commitment.to_bytes().into_inner())],
        vec![FixedBytes::new(proof.to_bytes().into_inner())]
    );

    // TODO: Gas Estimation Optimization
    let fee = provider.get_gas_price().await?;
    let max_fee = provider.get_max_priority_fee_per_gas().await?;

    let tx = TxEip4844 {
        chain_id: 17000, // Holesky
        nonce: nonce.to_string().parse().unwrap(),
        gas_limit: 21_000,
        max_fee_per_gas: 600_000_000_000,//fee.to_string().parse()?, // max (baseFee + priorityFee) refunds rest
        max_priority_fee_per_gas: 6_000_000_000,//max_fee.to_string().parse()?, 
        to: addr, 
        value: U256::from(0),
        access_list: AccessList(vec![]),
        blob_versioned_hashes: sidecar.versioned_hashes().collect(),
        max_fee_per_blob_gas: 30_000_000_000,
        input: bytes!(),
    };

    // Sign and submit 
    let mut variant = TxEip4844Variant::from((tx, sidecar));
    let signature = wallet.sign_transaction(&mut variant).await?;
    let tx_signed = variant.into_signed(signature);
    let tx_envelope: TxEnvelope = tx_signed.into();
    let encoded = tx_envelope.encoded_2718();

    let result = provider.send_raw_transaction(&encoded).await?;
    println!("{:?}", result);
    Ok(())
}
