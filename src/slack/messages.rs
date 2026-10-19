use chrono::NaiveDate;

pub fn challenge(name: &str, description: &str) -> String {
    format!("New limited time challenge just dropped:\n*{name}*\n{description}")
}

pub fn round(ship_by: NaiveDate) -> String {
    format!(
        "The next fulfillment round is coming up! Make sure you ship your project by *{}* in order to get your prizes sooner",
        ship_by.format("%B %-d")
    )
}

pub fn snapshot() -> String {
    "You've tracked enough time to send a Snapshot today!".into()
}

pub fn ship_approved(project: &str, mips: i32) -> String {
    format!("*{project}* was approved! You earned {mips} Mips.")
}

pub fn ship_rejected(project: &str) -> String {
    format!("*{project}* wasn't approved this time. Check your dashboard for details.")
}

pub fn inactivity() -> String {
    "We haven't seen you code in a week. Come back and keep building!".into()
}

pub fn draft_waiting(kind: &str, admin_url: &str) -> String {
    format!("A new {kind} message is waiting to be sent. <{admin_url}|Review it in the admin dashboard>")
}

pub fn welcome() -> String {
    "Hey there! I'm Steve the Sprite, you might remember me from your Mipmap onboarding :D\n\nI'll be communicating all sorts of information about the program. If you've got any questions, check out #mipmap!".into()
}
