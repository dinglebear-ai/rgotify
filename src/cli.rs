pub mod doctor;
pub mod setup;

use anyhow::{Result, bail};
use serde_json::Value;

use gotify_mcp::app::GotifyService;

pub enum CliCommand {
    Health,
    Version,
    Me,
    Messages {
        app_id: Option<i64>,
        limit: Option<i64>,
        since: Option<i64>,
    },
    Applications,
    Clients,
    Send {
        message: String,
        title: Option<String>,
        priority: Option<i64>,
    },
    DeleteMessage {
        id: i64,
        confirm: bool,
    },
    DeleteAllMessages {
        confirm: bool,
    },
    CreateApplication {
        name: String,
        description: Option<String>,
        default_priority: Option<i64>,
    },
    DeleteApplication {
        app_id: i64,
        confirm: bool,
    },
    UpdateApplication {
        app_id: i64,
        name: Option<String>,
        description: Option<String>,
        default_priority: Option<i64>,
    },
    CreateClient {
        name: String,
    },
    DeleteClient {
        client_id: i64,
        confirm: bool,
    },
    Doctor,
}

impl CliCommand {
    pub fn parse(args: &[String]) -> Result<(Self, bool)> {
        let json = args.iter().any(|a| a == "--json");
        let confirm = args.iter().any(|a| a == "--confirm");
        let rest: Vec<&str> = args
            .iter()
            .filter(|a| a.as_str() != "--json" && a.as_str() != "--confirm")
            .map(String::as_str)
            .collect();

        let cmd = match rest.as_slice() {
            ["health"] => Self::Health,
            ["version"] => Self::Version,
            ["me"] => Self::Me,
            ["messages", ..] => Self::Messages {
                app_id: flag_i64(&rest, "--app-id")?,
                limit: flag_i64(&rest, "--limit")?,
                since: flag_i64(&rest, "--since")?,
            },
            ["applications"] => Self::Applications,
            ["clients"] => Self::Clients,
            ["send", msg, ..] => Self::Send {
                message: msg.to_string(),
                title: flag_str(&rest, "--title"),
                priority: flag_i64(&rest, "--priority")?,
            },
            ["delete", "message", id] | ["delete-message", id] => Self::DeleteMessage {
                id: id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("message id must be an integer"))?,
                confirm,
            },
            ["delete", "all"] | ["delete-all"] => Self::DeleteAllMessages { confirm },
            ["create", "app", name, ..] | ["create-app", name, ..] => Self::CreateApplication {
                name: name.to_string(),
                description: flag_str(&rest, "--description"),
                default_priority: flag_i64(&rest, "--priority")?,
            },
            ["update", "app", id, ..] | ["update-app", id, ..] => Self::UpdateApplication {
                app_id: id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("app_id must be an integer"))?,
                name: flag_str(&rest, "--name"),
                description: flag_str(&rest, "--description"),
                default_priority: flag_i64(&rest, "--priority")?,
            },
            ["delete", "app", id, ..] | ["delete-app", id, ..] => Self::DeleteApplication {
                app_id: id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("app_id must be an integer"))?,
                confirm,
            },
            ["create", "client", name] | ["create-client", name] => Self::CreateClient {
                name: name.to_string(),
            },
            ["delete", "client", id] | ["delete-client", id] => Self::DeleteClient {
                client_id: id
                    .parse()
                    .map_err(|_| anyhow::anyhow!("client_id must be an integer"))?,
                confirm,
            },
            ["doctor"] => Self::Doctor,
            other => bail!(
                "unknown command: {}\n\nRun `rgotify --help` for usage.",
                other.join(" ")
            ),
        };
        Ok((cmd, json))
    }
}

fn flag_str(args: &[&str], flag: &str) -> Option<String> {
    let pos = args.iter().position(|a| *a == flag)?;
    args.get(pos + 1).map(|s| s.to_string())
}

fn flag_i64(args: &[&str], flag: &str) -> Result<Option<i64>> {
    let Some(pos) = args.iter().position(|a| *a == flag) else {
        return Ok(None);
    };
    let val = args
        .get(pos + 1)
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))?;
    val.parse::<i64>()
        .map(Some)
        .map_err(|_| anyhow::anyhow!("{flag}: expected integer, got {val:?}"))
}

pub async fn run(service: &GotifyService, cmd: CliCommand, json: bool) -> Result<()> {
    // Doctor does not need the service — it reads raw env and checks the system.
    if let CliCommand::Doctor = cmd {
        let port = gotify_mcp::config::Config::load()
            .map(|c| c.mcp.port)
            .unwrap_or(40020);
        return doctor::run_doctor(port, json).await;
    }

    let (label, data) = match cmd {
        CliCommand::Doctor => unreachable!("handled above"),
        CliCommand::Health => ("health", service.health().await?),
        CliCommand::Version => ("version", service.version().await?),
        CliCommand::Me => ("me", service.me().await?),
        CliCommand::Messages {
            app_id,
            limit,
            since,
        } => (
            "messages",
            service.messages(app_id, limit, since, None, None).await?,
        ),
        CliCommand::Applications => ("applications", service.applications(None).await?),
        CliCommand::Clients => ("clients", service.clients().await?),
        CliCommand::Send {
            ref message,
            ref title,
            priority,
        } => (
            "send",
            service
                .send(message, title.as_deref(), priority, None)
                .await?,
        ),
        CliCommand::DeleteMessage { id, confirm } => {
            ("delete_message", service.delete_message(id, confirm).await?)
        }
        CliCommand::DeleteAllMessages { confirm } => {
            ("delete_all", service.delete_all_messages(confirm).await?)
        }
        CliCommand::CreateApplication {
            ref name,
            ref description,
            default_priority,
        } => (
            "create_application",
            service
                .create_application(name, description.as_deref(), default_priority)
                .await?,
        ),
        CliCommand::UpdateApplication {
            app_id,
            ref name,
            ref description,
            default_priority,
        } => (
            "update_application",
            service
                .update_application(
                    app_id,
                    name.as_deref(),
                    description.as_deref(),
                    default_priority,
                )
                .await?,
        ),
        CliCommand::DeleteApplication { app_id, confirm } => (
            "delete_application",
            service.delete_application(app_id, confirm).await?,
        ),
        CliCommand::CreateClient { ref name } => {
            ("create_client", service.create_client(name).await?)
        }
        CliCommand::DeleteClient { client_id, confirm } => (
            "delete_client",
            service.delete_client(client_id, confirm).await?,
        ),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        print_human(label, &data);
    }
    Ok(())
}

fn print_human(cmd: &str, data: &Value) {
    match cmd {
        "health" => fmt_health(data),
        "version" => fmt_version(data),
        "me" => fmt_me(data),
        "messages" => fmt_messages(data),
        "applications" => fmt_applications(data),
        "clients" => fmt_clients(data),
        "send" => fmt_send(data),
        _ => println!("{}", serde_json::to_string_pretty(data).unwrap_or_default()),
    }
}

fn fmt_health(data: &Value) {
    let db = str_val_or(&data["database"], "?");
    let msgs = str_val_or(&data["health"], data["status"].as_str().unwrap_or("?"));
    println!("Health: {msgs}  DB: {db}");
}

fn fmt_version(data: &Value) {
    println!("Version:  {}", str_val_or(&data["version"], "?"));
    println!("Commit:   {}", str_val_or(&data["commit"], "?"));
    println!("Built at: {}", str_val_or(&data["buildDate"], "?"));
}

fn fmt_me(data: &Value) {
    println!("ID:    {}", data["id"].as_i64().unwrap_or(0));
    println!("Name:  {}", str_val_or(&data["name"], "?"));
    println!("Admin: {}", data["admin"].as_bool().unwrap_or(false));
}

fn fmt_messages(data: &Value) {
    let msgs = match data["messages"].as_array() {
        Some(m) => m,
        None => {
            // direct array response
            if let Some(arr) = data.as_array() {
                print_msg_table(arr);
                return;
            }
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    print_msg_table(msgs);
    if let Some(paging) = data.get("paging") {
        println!(
            "\nPage size: {}  Total: {}",
            paging["size"].as_i64().unwrap_or(0),
            paging["size"].as_i64().unwrap_or(0)
        );
    }
}

fn print_msg_table(msgs: &[Value]) {
    println!(
        "{:>6}  {:>4}  {:<20}  {:<32}  {}",
        "ID", "PRI", "APP", "TITLE", "MESSAGE"
    );
    for m in msgs {
        let id = m["id"].as_i64().unwrap_or(0);
        let pri = m["priority"].as_i64().unwrap_or(0);
        let app = m["appid"]
            .as_i64()
            .map(|i| format!("app:{i}"))
            .unwrap_or_else(|| "--".into());
        let title = str_val_or(&m["title"], "--");
        let message = str_val_or(&m["message"], "");
        let preview: String = message.chars().take(60).collect();
        println!("{id:>6}  {pri:>4}  {app:<20}  {title:<32}  {preview}");
    }
    println!("\n{} message(s)", msgs.len());
}

fn fmt_applications(data: &Value) {
    let apps = match data.as_array() {
        Some(a) => a,
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    println!(
        "{:>4}  {:<32}  {:<8}  {}",
        "ID", "NAME", "PRIORITY", "DESCRIPTION"
    );
    for a in apps {
        println!(
            "{:>4}  {:<32}  {:<8}  {}",
            a["id"].as_i64().unwrap_or(0),
            str_val_or(&a["name"], "?"),
            a["defaultPriority"].as_i64().unwrap_or(0),
            str_val_or(&a["description"], ""),
        );
    }
    println!("\n{} application(s)", apps.len());
}

fn fmt_clients(data: &Value) {
    let clients = match data.as_array() {
        Some(c) => c,
        None => {
            println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
            return;
        }
    };
    println!("{:>4}  {:<32}  {}", "ID", "NAME", "TOKEN");
    for c in clients {
        println!(
            "{:>4}  {:<32}  {}",
            c["id"].as_i64().unwrap_or(0),
            str_val_or(&c["name"], "?"),
            str_val_or(&c["token"], "--"),
        );
    }
    println!("\n{} client(s)", clients.len());
}

fn fmt_send(data: &Value) {
    if let Some(id) = data["id"].as_i64() {
        println!("Sent — message ID: {id}");
    } else {
        println!("{}", serde_json::to_string_pretty(data).unwrap_or_default());
    }
}

fn str_val_or<'a>(v: &'a Value, fallback: &'a str) -> &'a str {
    v.as_str().unwrap_or(fallback)
}
