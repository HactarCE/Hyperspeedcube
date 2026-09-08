//! Auth flow example.

#![allow(unused_crate_dependencies)]

use hsc_leaderboards;

const LOCALHOST: &str = "http://localhost:3000"; // when running leaderboards locally

fn main() -> Result<(), hsc_leaderboards::Error> {
    let auth_flow = hsc_leaderboards::AuthFlow::new(LOCALHOST);
    println!("Please open this URL in your browser and confirm authentication:");
    println!("{}", auth_flow.browser_url());
    println!();
    println!("Polling ...");
    let token = auth_flow.poll_until_done()?;
    println!("Received token: {token}");
    println!("Fetching user info ...");
    let leaderboards = hsc_leaderboards::Leaderboards::new(LOCALHOST, token)?;
    println!("Success!");
    println!();
    println!("{}", serde_json::to_string_pretty(&leaderboards).unwrap());
    Ok(())
}
