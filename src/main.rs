extern crate libc;

use std::io::{self, Write};
use std::ffi::*;
use libc::*;
use std::process::Command;

#[derive(Debug)]
struct CommandLine {
	args: Vec<String>,
	if_continue: bool,
}

#[derive(Debug)]
struct History {
	hist: Vec<String>,
	jobs: Vec<(i32, Vec<String>)>,
}

fn delete_empty(args: Vec<String>) -> Vec<String>{
	let mut deleted: Vec<String> = Vec::new();
	for arg in args {
		if arg!="".to_string() {
			deleted.push(arg);
		}
	}
	deleted
}

fn parse_cmd(mut cmd_line: String,
			 history: &mut History) 
	-> CommandLine{

	let mut command: CommandLine = CommandLine{  
		args: Vec::new(),
		if_continue: false,
	};
	history.hist.push(cmd_line.clone());
	// Throw the \n out.
	match cmd_line.pop(){
		// Or we are at EOF.
		None	=> safe_exit(0),
		Some(_)	=> { },
	}
	// Next find '&'
	match cmd_line.pop(){
		// Command with only \n (Null command).
		None	=> return command,
		Some(x) => match x {
			'&' => {
				command.if_continue = true;
				// Basic assumption: 
				// 		if and only if when a command ends with '&',
				//		it can be a target of command "jobs".
			},
			 _  => cmd_line.push(x),
		},
	};
	command.args = delete_empty(
			  	cmd_line.clone().split(' ')
					  	.map(|x| x.to_string()).collect());
	command
}

fn safe_exit(exit_code: i32){
	unsafe{ exit(exit_code) }
}

fn safe_kill(pid: String){
	// Just learned this way to convert a String to i32 from stackoverflow.
	// http://stackoverflow.com/questions/27043268/convert-a-string-to-int-in-rust
	let pid = pid.parse::<i32>().unwrap();
	if unsafe { kill(pid, SIGTERM) } == -1 {
		println!("KillError: failed to kill the process (pid:{})", pid);
	}
}

fn safe_chdir(curr: String, dest: String){
	// The real destination can be a folder in current directory,
	// or a totally new directory defined by user, we have to find out.
	let real_dest = match dest.as_bytes()[0]{
		// Commands starting with a "/", whose ascii is 47,
		// in which case, dest is the real dest.
		47	=> dest,
		// Or, dest just points a folder in current directory.
		_	=> curr + "/" + &dest,
	};
	// Turn to CString for ffi.
	let c_dest = CString::new(real_dest.as_bytes())
					.unwrap().as_ptr();
	// Change directory.
	match unsafe { chdir(c_dest) } {
	// According to API, 0 means success, -1 fail.
		0 => { },
		_ => println!("ChDirError: failed to change directory."),
	}
}
fn print_jobs (mut history: &mut History){
	for &(pid,ref job) in &history.jobs{
		;
	}
}
fn print_history(history: &History){
	for i in 0..history.hist.len()-1{
		print!("{:5}  {}", i+1, history.hist[i]);
	}
}

fn external_cmd(command: CommandLine, 
				mut history: &mut History){
	match unsafe { fork() } {
		// Child
		0 	=> {
			// Convert cmd(args[0]) and args into style,
			// and remove it by the way.
			let cmd = command.args[0].clone();
			let c_prog = CString::new(cmd.as_str()).unwrap();
			let mut c_args: Vec<_> = 
				command.args.iter()
							.map(|x| CString::new(x.as_str())
									.unwrap().as_ptr())
							.collect();
			c_args.push(std::ptr::null());

			match unsafe{ execvp(c_prog.as_ptr(), c_args.as_ptr()) }{
				-1 => println!("ExecError: failed to execute."),
				// Should execv(*const i8, *const *const i8) work properly, 
				// the following line won't print.
				_  => println!("ExecError: your computer's down 
								cause something crazy thing happened"),
			};		
		},
		// Error.
		-1	=> { println!("ForkError: failed to fork."); },
		// Parent.
		pid	=> {
			if !command.if_continue {
				// I'm not sure why should I cast it as *mut i32 here,
				// Nor it's downsides.
				unsafe{ wait(pid as *mut i32); };
			} else {
				// The existence of '&'
				history.jobs.push((pid, command.args));
			}
		},
	}
}

fn main() {
	let mut history: History = History{ hist: Vec::new(), jobs: Vec::new() };

	loop{
		print!("$ ");
		io::stdout().flush().unwrap();
		// Ensure stdout is printed immediately.

		let mut cmd_line = String::new();
		match io::stdin().read_line(&mut cmd_line){
		// Catch possible errors here.
			// Once nothing read(EOF), exit.
			Ok(_)  => { },
			Err(_) => { println!("ReadLineError: failed to read."); continue; },
		}
		let command = parse_cmd(cmd_line, &mut history);
		execute_cmd(command, &mut history);
	}
}

fn execute_cmd(command: CommandLine, mut history: &mut History){
	let cmd: String;
	// Test if it's null command
	match command.args.len() {
		0	=> return,
		_	=> cmd = command.args[0].clone(),
	}
	match cmd.as_ref() {
		"exit"	=> safe_exit(0),
		"pwd"	=> println!("{}", get_directory()),
		"cd"	=> { 
			let dest: String = {
				match command.args.len() {
					// In real bash, cd without parameters returns ~.
					1 => "/home".to_string(),
					// and real bash won't care if there are more parameters. 
					_ => command.args[1].clone(),
				}
			};
			safe_chdir(get_directory(), dest);
		},
		"history"	=> print_history(&history),
		"jobs"		=> print_jobs(&mut history),
		"kill"		=> match command.args.len(){
			1 => println!("KillError: no pid given."),
			_ => safe_kill(command.args[1].clone()),
		},
		_			=> { external_cmd(command, &mut history); },
	}
}

fn get_directory() -> String{
	let mut temp = String::from_utf8( 
			Command::new("pwd")
				.output().unwrap().stdout)
			.unwrap();
	// Discrad the last '\n'.
	temp.pop();	
	temp
}
