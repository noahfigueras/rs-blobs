use alloy::{
    network::EthereumSigner,
    providers::{ Provider, ProviderBuilder },
    rpc::types::eth::TransactionRequest,
    signers::wallet::LocalWallet,
    primitives::Address,
    consensus::{
        SidecarBuilder,
        SimpleCoder,
    },
};

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let wallet: LocalWallet = env!("PK").parse().unwrap();
    let addr = wallet.address();
    let signer = EthereumSigner::from(wallet);
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .signer(signer)
        .on_http(env!("RPC_URL").parse().unwrap()).unwrap();

    let blob_data = "---- Hello Blobs ----".as_bytes();
    let sidecar = SidecarBuilder::<SimpleCoder>::from_slice(&blob_data)
        .build().unwrap();

    let tx = TransactionRequest {
        from: Some(addr),
        to: Some(Address::ZERO),
        blob_versioned_hashes: Some(sidecar.versioned_hashes().collect()),
        sidecar: Some(sidecar),
        ..Default::default()
    };

    let tx = provider.send_transaction(tx).await.unwrap();
    let receipt = tx.get_receipt().await.unwrap();
    println!("{:?}", receipt);

    Ok(())
}
