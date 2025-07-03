use alloy::signers::ledger::{HDPath, LedgerError, LedgerSigner};
use anyhow::{bail, Result};

/// Try to obtain a ledger signer
///
/// Handles some common errors by prompting the user.
pub async fn connect_ledger(account_index: usize) -> Result<LedgerSigner> {
    let mut attempt = 1;
    let max_attempts = 20;
    Ok(loop {
        match LedgerSigner::new(HDPath::LedgerLive(account_index), None).await {
            Ok(signer) => break signer,
            Err(err) => {
                match err {
                    // Sadly, at this point, if we keep the app running unlocking the
                    // ledger does not make it show up.
                    LedgerError::LedgerError(ref ledger_error) => {
                        bail!("Error: {ledger_error:#}. Please unlock ledger and try again")
                    },
                    LedgerError::UnexpectedNullResponse => {
                        eprintln!(
                            "Failed to access ledger {attempt}/{max_attempts}: {err:#}, please \
                             unlock ledger and open the Ethereum app"
                        );
                    },
                    _ => {
                        bail!("Unexpected error accessing the ledger device: {err:#}")
                    },
                };
                if attempt >= max_attempts {
                    bail!("Failed to create Ledger signer after {max_attempts} attempts");
                }
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            },
        }
    })
}
