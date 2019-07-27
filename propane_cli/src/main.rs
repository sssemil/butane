use chrono::Utc;
use clap;
use clap::{Arg, ArgMatches};
use propane::migrations::Migrations;
use propane::{db, migrations};
use std::path::PathBuf;

type Result<T> = std::result::Result<T, failure::Error>;

fn main() {
    let args = clap::App::new("propane")
        .version(env!("CARGO_PKG_VERSION"))
        .author("James Oakley <james@electronstudio.org>")
        .about("Manages propane database migrations")
        .subcommand(
            clap::SubCommand::with_name("init")
                .about("Initialize the database")
                .arg(
                    Arg::with_name("BACKEND")
                        .required(true)
                        .index(1)
                        .help("Database backend to use"),
                )
                .arg(
                    Arg::with_name("CONNECTION")
                        .required(true)
                        .index(2)
                        .help("Database connection string. Format depends on backend"),
                ),
        )
        .subcommand(
            clap::SubCommand::with_name("makemigration")
                .about("Used for configuration")
                .arg(
                    Arg::with_name("name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .value_name("NAME")
                        .help("Name to use for the migration"),
                ),
        )
        .subcommand(clap::SubCommand::with_name("migrate"))
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .get_matches();
    match args.subcommand() {
        ("init", sub_args) => handle_error(init(sub_args)),
        ("makemigration", sub_args) => handle_error(make_migration(sub_args)),
        ("migrate", _) => handle_error(migrate()),
        (cmd, _) => eprintln!("Unknown command {}", cmd),
    }
}

fn default_name() -> String {
    Utc::now().format("%Y%m%d_%H%M%S%3f").to_string()
}

fn init<'a>(args: Option<&ArgMatches<'a>>) -> Result<()> {
    let args = args.unwrap();
    let name = args.value_of("BACKEND").unwrap();
    let connstr = args.value_of("CONNECTION").unwrap();
    if db::get_backend(name).is_none() {
        eprintln!("Unknown backend {}", name);
        std::process::exit(1);
    };

    let spec = db::ConnectionSpec::new(name, connstr);
    db::connect(&spec)?; // ensure we can
    spec.save(&base_dir()?)?;

    Ok(())
}

fn make_migration<'a>(args: Option<&ArgMatches<'a>>) -> Result<()> {
    let name = args
        .and_then(|a| a.value_of("name").and_then(|s| Some(s.to_string())))
        .unwrap_or_else(|| default_name());
    let ms = get_migrations()?;
    let m = ms.create_migration_sql(
        db::sqlite_backend(),
        &name,
        ms.get_latest(),
        &ms.get_current(),
    )?;
    match m {
        Some(m) => println!("Created migration {}", m.get_name()),
        None => println!("No changes to migrate"),
    }
    Ok(())
}

fn migrate() -> Result<()> {
    let spec = db::ConnectionSpec::load(&base_dir()?)?;
    let conn = db::connect(&spec)?;
    let to_apply = get_migrations()?.get_unapplied_migrations(&conn);
    for m in to_apply {
        println!("Applying migration {}", m.get_name());
        m.apply(&conn)?;
    }
    Ok(())
}

fn get_migrations() -> Result<Migrations> {
    Ok(migrations::from_root(base_dir()?.join("migrations")))
}

fn base_dir() -> Result<PathBuf> {
    std::env::current_dir()
        .map(|d| d.join("propane"))
        .map_err(|e| e.into())
}

fn handle_error(r: Result<()>) {
    match r {
        Err(e) => eprintln!("Encountered unexpectd error: {}", e),
        _ => (),
    }
}
