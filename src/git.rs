//! Checkout the index/stage once for each toolchain.
use super::Checkout;
use failure::Error;
use git2::build::CheckoutBuilder;
use git2::Repository;

/// Checkout the index/stage once for each toolchain.
pub fn checkout_index<'checkout, I>(
    checkouts: I,
    mut finished_callback: impl FnMut(&'checkout Checkout),
) -> Result<bool, Error>
where
    I: IntoIterator<Item = &'checkout Checkout>,
{
    let repo = Repository::open_from_env()?;

    let mut checkout_success = true;
    for checkout in checkouts {
        checkout.progress.set_message("checking out");
        std::fs::create_dir_all(&checkout.working_dir)?; // TODO isolate
        let mut ckopt = CheckoutBuilder::new();
        ckopt.target_dir(&checkout.working_dir);
        ckopt.recreate_missing(true);

        if let Err(e) = repo.checkout_index(None, Some(&mut ckopt)) {
            checkout
                .progress
                .finish_with_message(&format!("checkout error: {}", e));
            checkout_success = false;
        } else {
            checkout
                .progress
                .set_message("checked out, waiting on available worker");
            checkout.progress.inc(1);
            finished_callback(checkout);
        }
    }

    Ok(checkout_success)
}
