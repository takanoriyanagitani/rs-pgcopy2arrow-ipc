use std::io;
use std::process::ExitCode;

use io::BufWriter;
use io::Write;

use arrow_schema::SchemaRef;
use rs_pgcopy2rbat::arrow_schema;

use rs_pgcopy2arrow_ipc::reader2pgcopy2ipc2writer;
use rs_pgcopy2arrow_ipc::str2schema;

use rs_pgcopy2arrow_ipc::BAT_SIZE_DEFAULT;
use rs_pgcopy2arrow_ipc::SPLIT_CHAR_FOR_FIELD;
use rs_pgcopy2arrow_ipc::SPLIT_CHAR_FOR_SCHEMA;

fn io_envkey2val(key: &'static str) -> impl FnMut() -> Result<String, io::Error> {
    move || std::env::var(key).map_err(io::Error::other)
}

fn string2char(s: String) -> Result<char, io::Error> {
    s.chars()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty string got"))
}

fn string2usize(s: String) -> Result<usize, io::Error> {
    str::parse(&s).map_err(io::Error::other)
}

fn io_bind<F, G, T, U, E>(mut iofn: F, pure: G) -> impl FnMut() -> Result<U, E>
where
    F: FnMut() -> Result<T, E>,
    G: Fn(T) -> Result<U, E>,
{
    move || {
        let t: T = iofn()?;
        pure(t)
    }
}

fn io_split4schema() -> impl FnMut() -> Result<char, io::Error> {
    io_bind(io_envkey2val("ENV_SPLIT4SCHEMA"), string2char)
}

fn io_split4field() -> impl FnMut() -> Result<char, io::Error> {
    io_bind(io_envkey2val("ENV_SPLIT4FIELD"), string2char)
}

fn string2schema(
    split4schema: char,
    split4field: char,
) -> impl Fn(String) -> Result<SchemaRef, io::Error> {
    move |s: String| str2schema(&s, split4schema, split4field)
}

fn io_schema_string() -> impl FnMut() -> Result<String, io::Error> {
    io_envkey2val("ENV_SIMPLE_SCHEMA_DEF")
}

fn io_schema() -> impl FnMut() -> Result<SchemaRef, io::Error> {
    || {
        let split4schema: char = io_split4schema()().unwrap_or(SPLIT_CHAR_FOR_SCHEMA);
        let split4field: char = io_split4field()().unwrap_or(SPLIT_CHAR_FOR_FIELD);
        let str2sref = string2schema(split4schema, split4field);
        let sdef: String = io_schema_string()()?;
        str2sref(sdef)
    }
}

fn io_bat_size() -> impl FnMut() -> Result<usize, io::Error> {
    io_bind(io_envkey2val("ENV_BAT_SIZE"), string2usize)
}

fn io_main() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        let bat_size: usize = io_bat_size()().unwrap_or(BAT_SIZE_DEFAULT);
        let sref: SchemaRef = io_schema()()?;

        let rdr = io::stdin().lock();
        let o = io::stdout();
        let mut ol = o.lock();
        reader2pgcopy2ipc2writer(rdr, sref, bat_size, BufWriter::new(&mut ol))?;
        ol.flush()
    }
}

fn sub() -> Result<(), io::Error> {
    io_main()()
}

fn main() -> ExitCode {
    sub().map(|_| ExitCode::SUCCESS).unwrap_or_else(|e| {
        eprintln!("{e}");

        let split4schema: char = io_split4schema()().unwrap_or(SPLIT_CHAR_FOR_SCHEMA);
        let split4field: char = io_split4field()().unwrap_or(SPLIT_CHAR_FOR_FIELD);
        let schema_string: String = io_schema_string()().unwrap_or_default();

        let bat_size: usize = io_bat_size()().unwrap_or(BAT_SIZE_DEFAULT);

        eprintln!("ENV_SPLIT4SCHEMA: {split4schema}");
        eprintln!("ENV_SPLIT4FIELD: {split4field}");
        eprintln!("ENV_SIMPLE_SCHEMA_DEF: {schema_string}");
        eprintln!("ENV_BAT_SIZE: {bat_size}");

        ExitCode::FAILURE
    })
}
