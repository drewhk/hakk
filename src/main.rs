extern crate clap;
extern crate toml;

use clap::{Arg, App, AppSettings};
use std::fs;
use std::fs::File;
use std::io::{Write, Read, Result};
use std::env;
use toml::{Parser, Value};

struct Defaults<'a> {
	org: Option<&'a str>,
	scala: &'a str,
	akka: &'a str,
}

fn main() {
	let toml_path_opt = env::home_dir().and_then (|home| 
		home.to_str().map(|str| format!("{}/.hakk", str))		
	);

	// Read defaults file contents
	let toml = toml_path_opt.as_ref().and_then(|toml| read_toml(&toml).ok());
	// Parse defaults
	let toml = toml.and_then(|cont| Parser::new(&cont).parse()).map(|t| Value::Table(t));
	let defaults = toml.as_ref().and_then(|table| parse_toml(&table)).unwrap_or(
		Defaults { org: None, scala: "2.11.7", akka: "2.4.0" }
	);

	let matches = App::new("hakk")
		.setting(AppSettings::UnifiedHelpMessage)
		.about("Quick and simplistic Akka project generator")
		.version("0.1.0")		
		.arg(Arg::with_name("INSTALL")			
			.long("install")
			.help("Save passed arguments in ~/hakk (does not create project)")
			.conflicts_with("PROJECT_NAME")
			)
		.arg(Arg::with_name("PROJECT_NAME")
			.help("Name of the project (and the name of the target directory)")
			.required(true)
			.index(1)
			)				
		.arg(Arg::with_name("PROJECT_ORG")
			.help("Change organization of the project (defaults the project name)")
			.long("org")
			.required(false)
			.takes_value(true)
			)		
		.arg(Arg::with_name("PROJECT_VERSION")
			.long("ver")
			.help("Change the initial project version (defaults to 0.1-SNAPSHOT).")
			.conflicts_with("INSTALL")
			.takes_value(true))		
		.arg(Arg::with_name("AKKA_VERSION")
			.long("akka")
			.help("Set Akka version (defaults to value in ~/.hakk or 2.4.0)")
			.takes_value(true))
		.arg(Arg::with_name("SCALA_VERSION")
			.long("scala")
			.help("Set Scala version (defaults to value in ~/.hakk or 2.11.7)")
			.takes_value(true))		
		.arg(Arg::with_name("NO_GIT")
			.long("no-git")
			.help("Do not create a git repository")
			.conflicts_with("INSTALL")
			)
		.get_matches();

	let install_mode = matches.is_present("INSTALL");
		
	let project_v = matches.value_of("PROJECT_VERSION").unwrap_or("0.1-SNAPSHOT");
	let scala_v = matches.value_of("SCALA_VERSION").unwrap_or(defaults.scala);
	let akka_v = matches.value_of("AKKA_VERSION").unwrap_or(defaults.akka);
	let create_git = !matches.is_present("NO_GIT");

	if install_mode {		
		let new_defaults = Defaults {
			org: matches.value_of("PROJECT_ORG"),
			scala: scala_v,
			akka: akka_v,
		};

		let toml_path = toml_path_opt.expect("Cannot locate your home directory.");
		let toml_content = create_toml(&new_defaults);
		File::create(&toml_path)
			.expect("Could not create ~/.hakk")
			.write_all(toml_content.as_bytes())
			.expect("Could not write to ~/.hakk");

		println!("Wrote to file {}:", &toml_path);
		println!("{}", &toml_content);
		return;
	} 

	let name = matches.value_of("PROJECT_NAME").unwrap();
	let org = matches.value_of("PROJECT_ORG").or(defaults.org).unwrap_or(name);

	let build_sbt = create_build_sbt(name, org, project_v, scala_v, akka_v); 	

	let base_path = format!("./{}", name);

	// Y U NO WORK ???
	// let create_dir = |subpath: &str| {
	// 	fs::create_dir_all(base_path + subpath).expect(&format!("Could not create directory! {}", subpath))
	// };

	let dirs = vec![
		"", 
		"/src", 
		"/src/main",
		"/src/main/scala", 
		"/src/main/java",
		"/src/main/resources",
		"/src/test",
		"/src/test/scala", 
		"/src/test/java",
		"/src/test/resources",
	];

	for dir in &dirs { 
		fs::create_dir_all(format!("{}{}", base_path, dir))
			.expect(&format!("Could not create directory! {}", dir)) 
	};

	File::create(format!("{}{}", base_path, "/build.sbt"))
		.expect("Could not create build.sbt")
		.write_all(build_sbt.as_bytes())
		.expect("Could not write build.sbt");

	env::set_current_dir(&base_path).expect(&format!("Could not change directory to {}", base_path));

	if create_git {
		std::process::Command::new("git").arg("init")
			.status().expect("Executing git failed. Try option --no-git to disable this step.");
	}

}


fn parse_entry<'a>(table: &'a Value, key: &'a str) -> Option<&'a str> {
	table.lookup(key).and_then(|val| val.as_str())
}

fn parse_toml(table: &Value) -> Option<Defaults> {
	
	match (parse_entry(table, "versions.akka"), parse_entry(table, "versions.scala")) {
		(Some(ref a),  Some(ref s)) => Some(Defaults { 
			org: parse_entry(table, "metadata.organization"),
			scala: s,
			akka: a,
		}),
		_ => {
			println!("Found defaults file but could not parse contents!");
			println!("Contents were:\n{}", table);
			None
		}
	}
}

fn read_toml(toml: &str) -> Result<String> {
	let mut toml_file = try!(File::open(toml));
	let mut toml_content = String::new();
	try!(toml_file.read_to_string(&mut toml_content));
	Ok(toml_content)
}

fn create_build_sbt(
	project_name: &str, 
	project_organization: &str, 
	project_version: &str,
	scala_version: &str, 
	akka_version: &str) -> String {

	format!(include_str!("build.sbt.template"),
		project_name, project_organization, project_version, scala_version, akka_version)

}

fn create_toml(defaults: &Defaults) -> String {

	let mut result = String::new();

	match defaults.org {
		Some(org) => result.push_str(&format!("\
[metadata]		
organization = \"{}\"

", org)),
		_ => ()
	}

	result.push_str(&format!("\
[versions]
scala = \"{}\"
akka = \"{}\"", defaults.scala, defaults.akka));

	result
}
