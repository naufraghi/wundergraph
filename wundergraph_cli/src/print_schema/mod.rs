use crate::database::InferConnection;
use crate::infer_schema_internals::*;
use std::error::Error;
use std::io::Write;

mod print_helper;
use self::print_helper::*;

pub fn print<W: Write>(
    connection: &InferConnection,
    schema_name: Option<&str>,
    out: &mut W,
) -> Result<(), Box<dyn Error>> {
    let table_names = load_table_names(connection, schema_name)?;
    let foreign_keys = load_foreign_key_constraints(connection, schema_name)?;
    let foreign_keys =
        remove_unsafe_foreign_keys_for_codegen(connection, &foreign_keys, &table_names);

    let table_data = table_names
        .into_iter()
        .map(|t| load_table_data(connection, t))
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let definitions = TableDefinitions {
        tables: &table_data,
        include_docs: false,
        import_types: None,
    };
    let graphql = GraphqlDefinition {
        tables: &table_data,
        foreign_keys,
    };

    let mutations = GraphqlMutations {
        tables: &table_data,
    };
    writeln!(
        out,
        "use wundergraph::query_builder::types::{{HasMany, HasOne}};"
    )?;
    writeln!(out, "use wundergraph::scalar::WundergraphScalarValue;")?;
    writeln!(out, "use wundergraph::WundergraphEntity;")?;
    writeln!(out)?;
    writeln!(out, "{}", definitions)?;
    writeln!(out)?;
    writeln!(out, "{}", graphql)?;
    writeln!(out)?;
    writeln!(out, "{}", mutations)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(
        all(feature = "postgres", feature = "sqlite"),
        all(feature = "mysql", feature = "sqlite"),
        all(feature = "postgres", feature = "mysql")
    ))]
    compile_error!("Tests are only compatible with one backend");

    fn get_connection() -> InferConnection {
        use diesel::prelude::Connection;
        let db_url = std::env::var("DATABASE_URL").unwrap();
        #[cfg(feature = "postgres")]
        {
            let conn = diesel::pg::PgConnection::establish(&db_url).unwrap();
            conn.begin_test_transaction().unwrap();
            InferConnection::Pg(conn)
        }
        #[cfg(feature = "sqlite")]
        {
            let conn = diesel::sqlite::SqliteConnection::establish(&db_url).unwrap();
            conn.begin_test_transaction().unwrap();
            InferConnection::Sqlite(conn)
        }
        #[cfg(feature = "mysql")]
        {
            let conn = diesel::mysql::MysqlConnection::establish(&db_url).unwrap();
            conn.begin_test_transaction().unwrap();
            InferConnection::Mysql(conn)
        }
    }

    #[cfg(feature = "postgres")]
    const MIGRATION: &[&str] = &[
        "CREATE SCHEMA infer_test;",
        "CREATE TABLE infer_test.users(id SERIAL PRIMARY KEY, name TEXT NOT NULL);",
        r#"CREATE TABLE infer_test.posts(
            id SERIAL PRIMARY KEY,
            author INTEGER REFERENCES infer_test.users(id),
            title TEXT NOT NULL,
            content TEXT
        );"#,
        r#"CREATE TABLE infer_test.comments(
            id SERIAL PRIMARY KEY,
            post INTEGER REFERENCES infer_test.posts(id),
            commenter INTEGER REFERENCES infer_test.users(id),
            content TEXT NOT NULL
        );"#,
    ];

    #[cfg(feature = "sqlite")]
    const MIGRATION: &[&str] = &[
        "CREATE TABLE users(id INTEGER AUTOINCREMENT PRIMARY KEY, name TEXT NOT NULL);",
        r#"CREATE TABLEposts(
            id INTEGER AUTOINCREMENT PRIMARY KEY,
            author INTEGER REFERENCES users(id),
            title TEXT NOT NULL,
            content TEXT
        );"#,
        r#"CREATE TABLE infer_test.comments(
            id INTEGER AUTOINCREMENT PRIMARY KEY,
            post INTEGER REFERENCES infer_test.posts(id),
            commenter INTEGER REFERENCES infer_test.users(id),
            content TEXT NOT NULL
        );"#,
    ];

    fn setup_simple_schema(conn: &InferConnection) {
        use diesel::prelude::*;
        use diesel::sql_query;
        match conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(conn) => {
                for m in MIGRATION {
                    sql_query(*m).execute(conn).unwrap();
                }
            }
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(conn) => {
                for m in MIGRATION {
                    sql_query(m).execute(conn).unwrap();
                }
            }
        }
    }

    #[test]
    fn infer_schema() {
        let conn = get_connection();
        setup_simple_schema(&conn);

        let mut out = Vec::<u8>::new();

        print(&conn, Some("infer_test"), &mut out).unwrap();

        let s = String::from_utf8(out).unwrap();
        insta::assert_snapshot!(&s);
    }

    #[test]
    fn round_trip() {
        use std::fs::File;
        use std::io::{BufRead, BufReader, Read, Write};
        use std::process::Command;

        let conn = get_connection();
        setup_simple_schema(&conn);

        let tmp_dir = tempdir::TempDir::new("roundtrip_test").unwrap();

        let listen_url = "127.0.0.1:8001";
        Command::new("cargo")
            .arg("new")
            .arg("--bin")
            .arg("wundergraph_roundtrip_test")
            .current_dir(tmp_dir.path())
            .status()
            .unwrap();

        let api = tmp_dir.path().join("wundergraph_roundtrip_test/src/api.rs");
        let mut api_file = File::create(api).unwrap();
        print(&conn, Some("infer_test"), &mut api_file).unwrap();

        let main = tmp_dir
            .path()
            .join("wundergraph_roundtrip_test/src/main.rs");
        std::fs::remove_file(&main);
        let mut main_file = File::create(main).unwrap();

        let migrations = MIGRATION.iter().fold(String::new(), |mut acc, s| {
            acc += *s;
            acc += "\n";
            acc
        });

        #[cfg(feature = "postgres")]
        write!(
            main_file,
            include_str!("template_main.rs"),
            conn = "PgConnection",
            db_url = std::env::var("DATABASE_URL").unwrap(),
            migrations = migrations,
            listen_url = listen_url
        )
        .unwrap();

        let cargo_toml = tmp_dir.path().join("wundergraph_roundtrip_test/Cargo.toml");
        let mut cargo_toml_file = std::fs::OpenOptions::new()
            .write(true)
            .read(true)
            .create(false)
            .append(true)
            .open(cargo_toml)
            .unwrap();
        let current_root = env!("CARGO_MANIFEST_DIR");
        #[cfg(feature = "postgres")]
        {
            writeln!(
                cargo_toml_file,
                "{}",
                r#"diesel = {version = "1.4", features = ["postgres"]}"#
            )
            .unwrap();
            writeln!(
                cargo_toml_file,
                "wundergraph = {{path = \"{}/../wundergraph/\", features = [\"postgres\"] }}",
                current_root
            )
            .unwrap();
        }
        #[cfg(feature = "sqlite")]
        {
            writeln!(
                cargo_toml_file,
                "{}",
                r#"diesel = {version = "1.4", features = ["sqlite"]}"#
            )
            .unwrap();
            writeln!(
                cargo_toml_file,
                "wundergraph = {{path = \"{}/../wundergraph\", features = [\"sqlite\"] }}",
                current_root
            )
            .unwrap();
        }
        writeln!(cargo_toml_file, "{}", r#"juniper = "0.14""#).unwrap();
        writeln!(cargo_toml_file, "{}", r#"failure = "0.1""#).unwrap();
        writeln!(cargo_toml_file, "{}", r#"actix-web = "1""#).unwrap();
        writeln!(
            cargo_toml_file,
            "{}",
            r#"serde = {version = "1", features = ["derive"]}"#
        )
        .unwrap();
        writeln!(cargo_toml_file, "{}", r#"serde_json = "1""#).unwrap();

        std::mem::drop(conn);
        let mut child = Command::new("cargo")
            .arg("run")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::piped())
            .current_dir(tmp_dir.path().join("wundergraph_roundtrip_test"))
            .spawn()
            .unwrap();

        let mut r = BufReader::new(child.stderr.as_mut().unwrap());
        loop {
            let mut line = String::new();
            r.read_line(&mut line).unwrap();
            println!("{}", line.trim());

            if line.trim().starts_with("Running ") {
                break;
            }
            if line.trim().starts_with("error: ") {
                panic!("Failed to compile example application");
            }
        }

        println!("Started server");

        let client = reqwest::Client::new();
        std::thread::sleep(std::time::Duration::from_secs(1));

        let query = "{\"query\": \"{ Users { id  name  } } \"}";
        let mut r = client
            .post(&format!("http://{}/graphql", listen_url))
            .body(query)
            .header(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .send()
            .unwrap();
        insta::assert_json_snapshot!(r.json::<serde_json::Value>().unwrap());

        let mutation = r#"{"query":"mutation CreateUser {\n  CreateUser(NewUser: {name: \"Max\"}) {\n    id\n    name\n  }\n}","variables":null,"operationName":"CreateUser"}"#;
        let mut r = client
            .post(&format!("http://{}/graphql", listen_url))
            .body(mutation)
            .header(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .send()
            .unwrap();
        insta::assert_json_snapshot!(r.json::<serde_json::Value>().unwrap());

        let mut r = client
            .post(&format!("http://{}/graphql", listen_url))
            .body(query)
            .header(
                reqwest::header::CONTENT_TYPE,
                reqwest::header::HeaderValue::from_static("application/json"),
            )
            .send()
            .unwrap();
        insta::assert_json_snapshot!(r.json::<serde_json::Value>().unwrap());

        child.kill().unwrap();
        child.wait().unwrap();
    }
}
