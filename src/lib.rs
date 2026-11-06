use std::io;

use io::BufRead;
use io::Read;

use io::BufWriter;
use io::Write;

use rs_pgcopy2rbat::arrow_schema;

use arrow_schema::DataType;

use arrow_schema::Field;
use arrow_schema::Fields;

use arrow_schema::Schema;
use arrow_schema::SchemaRef;

use arrow_ipc::writer::StreamWriter;

use rs_pgcopy2rbat::Builders;
use rs_pgcopy2rbat::Column;

use rs_pgcopy2rbat::Header;

pub fn reader2swtr<R, W>(
    rdr: &mut R,
    sch: SchemaRef,
    builders: &mut Builders,
    dtypes: &[DataType],
    colbuf: &mut Column,
    bat_size: usize,
    mut wtr: StreamWriter<W>,
) -> Result<(), io::Error>
where
    R: Read,
    W: Write,
{
    let mut is_last_batch: bool = false;
    loop {
        if is_last_batch {
            break;
        }

        let pair =
            rs_pgcopy2rbat::reader2batch(rdr, sch.clone(), builders, dtypes, colbuf, bat_size)?;
        let (rbat, is_last) = pair;
        is_last_batch = is_last;
        let empty: bool = 0 == rbat.num_rows();
        if empty {
            break;
        }

        wtr.write(&rbat).map_err(io::Error::other)?;
    }

    let mut w: W = wtr.into_inner().map_err(io::Error::other)?;
    w.flush()
}

pub fn reader2pgcopy2ipc2writer<R, W>(
    mut rdr: R,
    sch: SchemaRef,
    bat_size: usize,
    wtr: W,
) -> Result<(), io::Error>
where
    R: BufRead,
    W: Write,
{
    Header::from_rdr(&mut rdr)?;
    let mut bldrs: Builders = Builders::from_schema(&sch);
    let idtyp = sch.fields().iter().map(|field| {
        let f: &Field = field;
        let dtyp: &DataType = f.data_type();
        dtyp.clone()
    });
    let dtyps: Vec<DataType> = idtyp.collect();
    let mut colbuf: Column = Column::default();
    let swtr: StreamWriter<W> = StreamWriter::try_new(wtr, &sch).map_err(io::Error::other)?;
    reader2swtr(
        &mut rdr,
        sch,
        &mut bldrs,
        &dtyps,
        &mut colbuf,
        bat_size,
        swtr,
    )
}

pub fn fields2schema<I>(ifield: I) -> SchemaRef
where
    I: Iterator<Item = Field>,
{
    let fields = Fields::from_iter(ifield);
    Schema::new(fields).into()
}

pub fn strings2field(name: &str, sdtyp: &str, snullable: &str) -> Result<Field, io::Error> {
    let nullable: bool = str::parse(snullable).unwrap_or_default();
    let dtyp: DataType = str::parse(sdtyp).map_err(io::Error::other)?;
    Ok(Field::new(name, dtyp, nullable))
}

pub fn str2field(s: &str, split: char) -> Result<Field, io::Error> {
    let mut splited = s.split(split);
    let name: &str = splited.next().unwrap_or_default();
    let sdtyp: &str = splited.next().unwrap_or_default();
    let snullable: &str = splited.next().unwrap_or_default();
    strings2field(name, sdtyp, snullable)
}

pub fn str2schema(s: &str, split4schema: char, split4field: char) -> Result<SchemaRef, io::Error> {
    let splited = s.split(split4schema);
    let mapd = splited.map(|sfield| str2field(sfield, split4field));
    let rvf: Result<Vec<Field>, _> = mapd.collect();
    let vf: Vec<Field> = rvf?;
    Ok(fields2schema(vf.into_iter()))
}

pub const SPLIT_CHAR_FOR_SCHEMA: char = ';';
pub const SPLIT_CHAR_FOR_FIELD: char = ',';

pub struct SimpleSchemaConfig {
    pub split_char4schema: char,
    pub split_char4field: char,
}

impl SimpleSchemaConfig {
    pub fn string2schema(&self, s: String) -> Result<SchemaRef, io::Error> {
        str2schema(&s, self.split_char4schema, self.split_char4field)
    }
}

impl Default for SimpleSchemaConfig {
    fn default() -> Self {
        Self {
            split_char4schema: SPLIT_CHAR_FOR_SCHEMA,
            split_char4field: SPLIT_CHAR_FOR_FIELD,
        }
    }
}

pub const BAT_SIZE_DEFAULT: usize = 8192;

pub struct Config {
    pub simple_schema_cfg: SimpleSchemaConfig,
    pub simple_schema_string: String,
    pub bat_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            simple_schema_cfg: SimpleSchemaConfig::default(),
            simple_schema_string: "".into(),
            bat_size: BAT_SIZE_DEFAULT,
        }
    }
}

impl Config {
    pub fn with_schema_string(mut self, neo: String) -> Self {
        self.simple_schema_string = neo;
        self
    }
}

impl Config {
    pub fn rdr2pgcopy2ipc2wtr<R, W>(self, rdr: R, wtr: W) -> Result<(), io::Error>
    where
        R: BufRead,
        W: Write,
    {
        let sch: SchemaRef = self
            .simple_schema_cfg
            .string2schema(self.simple_schema_string)?;
        reader2pgcopy2ipc2writer(rdr, sch, self.bat_size, wtr)
    }
}

impl Config {
    pub fn stdin2pgcopy2ipc2stdout(self) -> Result<(), io::Error> {
        let o = io::stdout();
        let mut ol = o.lock();
        self.rdr2pgcopy2ipc2wtr(io::stdin().lock(), BufWriter::new(&mut ol))?;
        ol.flush()
    }
}
