extern crate libc;

use std::io::{self, Write};
use std::ffi::*;
use libc::*;
use std::process::Command;

fn read_cmd(history: &mut Vec<String>) -> (String, Vec<String>, bool){
	
	let mut cmd_line = String::new();
	match io::stdin().read_line(&mut cmd_line){
	// Catch possible errors here.
		Ok(_)  => { },
		Err(_) => { println!("ReadLineError: failed to read."); },
	}
	history.push(cmd_line.clone());

	let mut if_continue: bool = false;
	let mut args: Vec<String> = Vec::new();

	let mut curr_argu = 0;
	let mut prev = ' ';
	for curr in cmd_line.chars(){
		match curr {
			'\n'=> { }
			' ' => { },
			'&' => { if_continue = true; },
			 _  => { 
			 	if (prev ==' ') || (prev =='&') {
			 		args.push(String::new());
			 		curr_argu += 1;
			 	}
			 	args[curr_argu-1].push(curr);
			 },
		};
		prev = curr;		
	}

    let mut iter = args.into_iter();
    let cmd = match iter.next(){
    	Some(temp)	=> temp,
    	None		=> "".to_string(),

    };
    let args: Vec<String> = iter.collect();
	(cmd, args, if_continue)
}

fn safe_exit(exit_code: i32){
	unsafe{ exit(exit_code) }
}

fn safe_chdir(mut curr: String, dest: String) -> Option<String>{
	// The real destination can be a folder in current directory,
	// or a totally new directory defined by user, we have to find out.
	let real_dest = match dest.as_bytes()[0]{
		// Commands starting with a "/", whose ascii is 47,
		// in which case, dest is the real dest.
		47	=> dest,
		// Or, dest just points a folder in current directory.
		_	=> {
			match dest.as_ref() {
				// But two more special case, . and ..
				"."		=> curr,
				".."	=> {
					// Thr first char of curr must be '/',
					// so unwrap is safe here.
					while curr.pop().unwrap() != '/'	{}
					match curr.len() {

						0 => "/".to_string(),
						_ => curr,
					}
				}
				_		=> curr + "/" + &dest,
			}
		}
	};

	let c_dest = CString::new(real_dest.as_bytes())
					.unwrap();

	match unsafe { chdir(c_dest.as_ptr()) } {
		0 => Some(real_dest),
		_ => None,
	}
}

fn print_history(history: &Vec<String>){
	for i in 0..history.len()-1{
		print!("{:5}  {}", i+1, history[i]);
	}
}

fn external_cmd(cmd: &String, 
				args: &Vec<String>, 
				if_continue: &bool){
}

fn main() {
	// Get current working directory.
	let mut directory: String = 
		String::from_utf8( Command::new("pwd")
							.output()
							.unwrap()
							.stdout
		).unwrap();
	// Discard the last \n.
	directory.pop();

	let mut history: Vec<String> = Vec::new();

	loop{
		print!("$ ");
		io::stdout().flush().unwrap();
		// Ensure stdout is printed immediately.

		let (cmd, args, if_continue) = read_cmd(&mut history);

		match cmd.as_ref() {
			"exit"	=> safe_exit(0),
			"pwd"	=> println!("{}", &directory),
			"clear"	=> { ; },
			"ls"	=> {	
				let curr = String::from_utf8( Command::new("ls")
								.output()
								.unwrap()
								.stdout
							).unwrap();
				for c in curr.split('\n')
							 .map(|x| x.to_string()){
					if c!="".to_string(){ 
						print!("    {}\n", c); 
					}
				}
			}
			"cd"	=> { 
				let dest: String = {
					match args.len() {
						// In real bash, cd without parameters returns ~.
						0 => "/home".to_string(),
						// and real bash won't care if there are more parameters. 
						_ => args[0].clone(),
					}
				};
				match safe_chdir(directory.clone(), dest){
					Some(result)=> directory = result,
					None		=> 
						println!("ChDirError: failed to change directory."),
				}
			},
			"history"	=> print_history(&history),
			_		=> external_cmd(&cmd, &args, &if_continue),
		}
	}
}

