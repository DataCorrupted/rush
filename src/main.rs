extern crate libc;

use std::io::{self, Write};
use std::ffi::*;
use libc::*;
use std::env;

#[derive(Debug)]
struct CommandLine {
	cmd: String,
	args: Vec<String>,
	if_continue: bool,
}
#[derive(Debug)]
struct History {
	hist: Vec<String>,
	jobs: Vec<(i32, Vec<String>)>,
}

fn split2vec(temp: &String, c: char) -> Vec<String>{
	temp.clone().split(c).map(|x| x.to_string()).collect()
}

fn parse_cmd(mut cmd_line: String,
			 history: &mut History) -> CommandLine{
	let mut command: CommandLine = CommandLine{  
		cmd: String::new(),
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
	cmd_line = str::replace(cmd_line.as_str(), "\t", " ").to_string();
	loop {
		// Next find '&',
		// and clear all the unnecessary tags like ' '.
		match cmd_line.pop(){
			// Command with only \n (Null command).
			None	=> return command,
			Some(x) => match x {
				'&' => { command.if_continue = true; },
				' ' => continue,
				 _  => { cmd_line.push(x); break; },
			},
		};
	}
	if split2vec(&cmd_line, '|').len() == 1 {
	// There is no '|' in the cmd_line
	// then split and ignore ""
		for arg in split2vec(&cmd_line, ' '){
			match arg.as_ref() {
				""	=> { },
				_	=> command.args.push(arg),
			}
		}
		command.cmd = match command.args.len(){
				0	=> "".to_string(),
				_	=> command.args[0].clone(),
		};
	} else {
	// else just take each part between '|' and take it as args
		command.cmd = "pipe".to_string();
		command.args = split2vec(&cmd_line, '|');	
	}
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
	//	println!("KillError: failed to kill the process (pid:{})", pid);
	}
}

fn safe_chdir(dest: String){
	// The real destination can be a folder in current directory,
	// or a totally new directory defined by user, 
	// we have use expand address to find out.
	let real_dest = get_absolute_path(dest);
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

fn safe_execvp(args: Vec<String>){
	let cmd = args[0].clone();
	// Convert cmd(args[0]) and args into C style,
	let c_prog = CString::new(cmd.as_bytes()).unwrap();
	// The following lines must be wrong
	// multiple args couldn't be translated to C style.
	// For example, ls -a -l will only take -l pary.
	let mut c_args: Vec<_> = 
		args.into_iter()
			.map(|x| CString::new(x.as_bytes())
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
}

/*
fn safe_pipe(command: CommandLine,
			 mut history: &mut History){
	println!("{:?}", command.args);
}*/

fn io_redirection(command: &CommandLine) -> Vec<String> {
	// Abstract I/O files in 
	let mut in_handle = -1;
	let mut out_handle = -1;
	let mut args: Vec<String> = Vec::new();
	let mut i = 0;
	loop {
	//for arg in command.args.clone(){
		let arg = command.args[i].clone();
		match arg.chars().nth(0).unwrap(){
			'<'|'>'	=> { 
				let file = match arg.len(){
					1	=> { i += 1; command.args[i].clone() },
					_	=> arg[1..].to_string(),
				};
				let file  = CString::new(
						get_absolute_path(file).as_str()
					).unwrap().as_ptr();
				// In case such file don't exist, we create it by fopen.
				create_file(file);
				match arg.chars().nth(0).unwrap(){
					'<'	=> { in_handle = unsafe { open(file, O_RDONLY) }; },
					'>' => { out_handle = unsafe { open(file, O_WRONLY) }; },
					 _ 	=> { },
				};
		
			},
			 _  => args.push(arg.clone()),
		}
		i += 1;
		if i == command.args.len() { break; }
	}
	unsafe { 
		if in_handle != -1{
			dup2(in_handle, 0); close(in_handle);
		}
		if out_handle != -1{
			dup2(out_handle, 1); close(out_handle); 
		}
	}
	args
}

fn external_cmd(command: CommandLine, 
				mut history: &mut History){
	match unsafe { fork() } {
		// Child
		0 	=> {
			let args = io_redirection(&command);
			safe_execvp(args);
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
	let mut history: History = 
		History{ hist: Vec::new(), jobs: Vec::new() };
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
	match command.cmd.as_ref() {
		""			=> return,
		"exit"		=> safe_exit(0),
		"pwd"		=> println!("{}", get_directory()),
		"cd"		=> { 
			match command.args.len() {
				// In real bash, cd without parameters returns ~.
				1 => println!("CdError: no directory given."),
				// and real bash won't care if there are more parameters. 
				_ => safe_chdir(command.args[1].clone()),
			}
		},
		//"pipe"		=> safe_pipe(command, &mut history),
		"history"	=> print_history(&history),
		"jobs"		=> print_jobs(&mut history),
		"kill"		=> match command.args.len(){
			1 => println!("KillError: no pid given."),
			_ => safe_kill(command.args[1].clone()),
		},
		_			=> { external_cmd(command, &mut history); },
	}
}

fn create_file(file: *const i8){ unsafe {
	let open_file = fopen(file, CString::new("a").unwrap().as_ptr()); 
	fclose(open_file);	
} }

fn get_directory() -> String{
	env::current_dir().unwrap().display().to_string()
}

fn get_absolute_path(dest: String) -> String {
	match dest.as_bytes()[0]{
		// Commands starting with a "/", whose ascii is 47,
		// in which case, dest is the real dest.
		47	=> dest,
		// Or, dest just points a folder in current directory.
		_	=> get_directory() + "/" + &dest,
	}
}

fn print_job(job: &Vec<String>){
	for i in 0..job.len(){
		print!("{}", job[i]);
		if i!=job.len()-1 { print!(" "); }
	}
	print!("\n");
}
fn print_jobs (history: &mut History){
	let mut temp: i32 = 1;
	for &(pid, ref job) in &history.jobs{
		match unsafe{ waitpid(pid, &mut temp, WNOHANG)} {
			0	=> { print_job(job); },
			_	=> { },
		}
	}
}

fn print_history(history: &History){
	for i in 0..history.hist.len()-1{
		print!("{:5}  {}", i+1, history.hist[i]);
	}
}
