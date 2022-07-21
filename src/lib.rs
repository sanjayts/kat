use clap::{App, Arg, ArgMatches};
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader};

use csv::StringRecord;
use std::ops::Range;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    selector: Selector,
    delimiter: u8, // This is a u8 because csv parser supports only byte delimiters!
}

#[derive(Debug)]
enum Selector {
    Bytes(Positions),
    Chars(Positions),
    Fields(Positions),
}

type Positions = Vec<Range<usize>>;

pub type CliResult<T> = Result<T, Box<dyn Error>>;

pub fn run(config: Config) -> CliResult<()> {
    for file in &config.files {
        match open(file.as_str()) {
            Err(e) => eprintln!("{}: {}", file, e),
            Ok(reader) => process_reader(reader, &config)?,
        }
    }
    Ok(())
}

fn process_reader(reader: impl BufRead, config: &Config) -> CliResult<()> {
    let unique_indices = |positions: &Positions| {
        // The clone below is needed because we can't collect a range we don't own and iter
        // gives us back the reference to a range. We could also make this work with into_iter
        // on positions but then our iter owns the data which is something we don't want!
        positions
            .iter()
            .flat_map(|r| r.clone().collect::<Vec<usize>>())
            .collect::<HashSet<usize>>()
    };

    match &config.selector {
        Selector::Bytes(positions) => {
            let all_pos = unique_indices(positions);
            for result in reader.lines() {
                let line = result?;
                let mut byte_buf = Vec::new();
                for (idx, b) in line.as_bytes().iter().enumerate() {
                    if all_pos.contains(&idx) {
                        byte_buf.push(*b);
                    }
                }
                // Since it's possible for us to pick off random bytes from a multi-byte seq
                // in a UTF-8 file, it's important to create a lossy string to avoid an error
                // at runtime.
                println!("{}", String::from_utf8_lossy(byte_buf.as_slice()));
            }
        }
        Selector::Chars(positions) => {
            let all_pos = unique_indices(positions);
            for result in reader.lines() {
                let line = result?;
                let mut char_buf = Vec::new();
                for (idx, c) in line.chars().enumerate() {
                    if all_pos.contains(&idx) {
                        char_buf.push(c);
                    }
                }
                let mut line = String::new();
                line.extend(char_buf);
                println!("{}", line);
            }
        }
        Selector::Fields(positions) => {
            let all_pos = unique_indices(positions);
            let mut csv_reader = csv::ReaderBuilder::new()
                .delimiter(config.delimiter)
                .has_headers(false)
                .from_reader(reader);
            let mut csv_writer = csv::WriterBuilder::new()
                .delimiter(config.delimiter)
                .from_writer(stdout());

            let mut printer = |record: &mut &StringRecord| -> CliResult<()> {
                for (idx, val) in record.iter().enumerate() {
                    if all_pos.contains(&idx) {
                        csv_writer.write_field(val)?;
                    }
                }
                csv_writer.write_record(None::<&[u8]>)?;
                Ok(())
            };

            for result in csv_reader.records() {
                let record = result?;
                printer(&mut &record)?;
            }
        }
    }
    Ok(())
}

fn open(file: &str) -> CliResult<Box<dyn BufRead>> {
    match file {
        "-" => Ok(Box::new(BufReader::new(stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(file)?))),
    }
}

pub fn parse_config(cmd_args: Vec<String>) -> CliResult<Config> {
    let matches = App::new("kat")
        .version("0.1.0")
        .author("sanjayts")
        .arg(
            Arg::new("delimiter")
                .value_name("DELIM")
                .short('d')
                .long("delimiter")
                .help("Use DELIM instead of TAB for delimiter")
                .default_value("\t")
                .takes_value(true)
                .multiple_values(false),
        )
        .arg(
            Arg::new("fields")
                .value_name("LIST")
                .short('f')
                .long("fields")
                .help("select only these fields")
                .takes_value(true)
                .conflicts_with_all(&["bytes", "characters"]),
        )
        .arg(
            Arg::new("characters")
                .value_name("LIST")
                .short('c')
                .long("characters")
                .help("select only these characters")
                .takes_value(true)
                .conflicts_with_all(&["bytes", "fields", "delimiter"]),
        )
        .arg(
            Arg::new("bytes")
                .value_name("LIST")
                .short('b')
                .long("bytes")
                .help("select only these bytes")
                .multiple_values(false)
                .conflicts_with_all(&["fields", "characters", "delimiter"]),
        )
        .arg(
            Arg::new("files")
                .value_name("FILE")
                .takes_value(true)
                .multiple_values(true)
                .default_value("-"),
        )
        .get_matches_from(cmd_args);

    let files = matches
        .get_many::<String>("files")
        .unwrap()
        .map(|s| s.to_owned())
        .collect();

    let delimiter = matches.get_one::<String>("delimiter").unwrap().to_owned();
    if delimiter.len() != 1 {
        return Err("kat: bad delimiter".into());
    }

    let selector = parse_selector(&matches)?;

    let config = Config {
        files,
        delimiter: delimiter.bytes().next().unwrap(),
        selector,
    };
    Ok(config)
}

fn parse_positions(arg: &str) -> CliResult<Positions> {
    let func = |v: &str| format!("kat: illegal list value: '{}'", v);
    if arg.is_empty() || arg.starts_with(',') || arg.ends_with(',') {
        return Err(func(arg).into());
    }

    let parse_num = |s: &str| -> CliResult<usize> {
        if s.starts_with('-') || s.starts_with('+') || s.ends_with('-') || s.ends_with('+') {
            return Err(func(s).into());
        }
        // Without the explicit cast below, map_err can't infer that we want a dyn Error
        let num = s.parse().map_err::<Box<dyn Error>, _>(|_| func(s).into())?;
        if num == 0 {
            Err("kat: list values may not include zero".into())
        } else {
            Ok(num)
        }
    };

    let mut positions = vec![];
    for part in arg.split(',') {
        if part.starts_with('-') || part.ends_with('-') {
            return Err(func(part).into());
        }

        let inner_parts = part.split('-').collect::<Vec<_>>();
        if inner_parts.len() == 1 {
            let n: usize = parse_num(inner_parts[0])?;
            positions.push((n - 1)..n);
        } else if inner_parts.len() == 2 {
            let start: usize = parse_num(inner_parts[0])?;
            let end: usize = parse_num(inner_parts[1])?;
            if end <= start {
                let msg = format!(
                    "First number in range ({}) must be lower than second number ({})",
                    start, end
                );
                return Err(msg.into());
            }
            positions.push((start - 1)..end);
        } else {
            return Err(func(part).into());
        }
    }
    Ok(positions)
}

fn parse_selector(matches: &ArgMatches) -> CliResult<Selector> {
    let extract_positions = |id| {
        matches
            .get_one::<String>(id)
            .map(|s| parse_positions(s.as_str()))
            .transpose()
    };

    let fields = extract_positions("fields")?;
    let chars = extract_positions("characters")?;
    let bytes = extract_positions("bytes")?;

    if let Some(positions) = fields {
        Ok(Selector::Fields(positions))
    } else if let Some(positions) = chars {
        Ok(Selector::Chars(positions))
    } else if let Some(positions) = bytes {
        Ok(Selector::Bytes(positions))
    } else {
        Err("Must have --fields, --bytes, or --chars".into())
    }
}

#[cfg(test)]
mod lib_tests {
    use crate::{parse_config, parse_positions};
    use std::{assert_eq, vec};

    #[test]
    fn test_no_args() {
        let cfg = parse_config(to_owned_arg_list(vec!["kat", "-b", "1"]));

        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert_eq!(cfg.delimiter, b'\t');
        assert_eq!(cfg.files, vec!["-"]);
    }

    #[test]
    fn test_custom_delim() {
        let args = to_owned_arg_list(vec!["kat", "-d", "x", "-f", "1", "a.txt", "b.txt"]);
        let cfg = parse_config(args);

        assert!(cfg.is_ok());
        let cfg = cfg.unwrap();
        assert_eq!(cfg.delimiter, b'x');
        assert_eq!(cfg.files, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn test_bad_delim() {
        let args = to_owned_arg_list(vec!["kat", "-d", "xxx", "-f", "1"]);
        let cfg = parse_config(args);

        assert!(cfg.is_err());
        assert_eq!(cfg.unwrap_err().to_string(), "kat: bad delimiter");
    }

    #[test]
    fn test_parse_positions() {
        let arg = "1";
        let positions = parse_positions(arg);
        assert!(positions.is_ok());
        assert_eq!(positions.unwrap(), vec![0..1]);

        let arg = "1-3";
        let positions = parse_positions(arg);
        assert!(positions.is_ok());
        assert_eq!(positions.unwrap(), vec![0..3]);

        let arg = "1-3,8-10";
        let positions = parse_positions(arg);
        assert!(positions.is_ok());
        assert_eq!(positions.unwrap(), vec![0..3, 7..10]);

        // The empty string is an error
        assert!(parse_positions("").is_err());

        // Zero is an error
        let res = parse_positions("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "kat: list values may not include zero"
        );

        let res = parse_positions("0-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "kat: list values may not include zero",
        );

        // A leading "+" is an error
        let res = parse_positions("+1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "kat: illegal list value: '+1'",
        );

        let res = parse_positions("+1-2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "kat: illegal list value: '+1'",
        );

        let res = parse_positions("+2");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "kat: illegal list value: '+2'",
        );

        // Any non-number is an error
        let res = parse_positions("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "kat: illegal list value: 'a'",);

        let res = parse_positions("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "kat: illegal list value: 'a'",);

        let res = parse_positions("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "kat: illegal list value: 'a'",);

        let res = parse_positions("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "kat: illegal list value: 'a'",);

        // Wonky ranges
        let res = parse_positions("-");
        assert!(res.is_err());

        let res = parse_positions(",");
        assert!(res.is_err());

        let res = parse_positions("1,");
        assert!(res.is_err());

        let res = parse_positions("1-");
        assert!(res.is_err());

        let res = parse_positions("1-1-1");
        assert!(res.is_err());

        let res = parse_positions("1-1-a");
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_positions("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_positions("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_positions("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_positions("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_positions("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_positions("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_positions("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_positions("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_positions("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_positions("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }

    fn to_owned_arg_list(args: Vec<&str>) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }
}
