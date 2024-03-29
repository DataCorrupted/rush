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
	jobs: Vec<(Vec<i32>, Vec<String>)>,
}

fn split2vec(temp: &String, c: char) -> Vec<String>{
	temp.clone().split(c).map(|x| x.to_string()).collect()
}

fn parse_cmd(mut cmd_line: String) -> CommandLine{
	let mut command: CommandLine = CommandLine{  
		cmd: String::new(),
		args: Vec::new(),
		if_continue: false,
	};

	cmd_line = str::replace(cmd_line.as_str(), "\n", " ").to_string();
	cmd_line = str::replace(cmd_line.as_str(), "\t", " ").to_string();

	loop {
		// Next find '&',
		// and clear all the unnecessary tags like ' '.
		match cmd_line.pop(){
			// Command with nothing (Null command).
			None	=> return command,
			Some(x) => match x {
				'&' => { command.if_continue = true; },
				' ' => continue,
				// We reached the rightmost byte and it's not a &,
				// so this stage is done.
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

fn safe_kill(pid: String){
	// Just learned this way to convert a String to i32 from stackoverflow.
	// http://stackoverflow.com/questions/27043268/convert-a-string-to-int-in-rust
	let pid = pid.parse::<i32>().unwrap();
	if unsafe { kill(pid, libc::SIGTERM) } == -1 {
		println!("KillError: failed to kill the process (pid:{})", pid);
	}
}

fn safe_chdir(dest: String){
	// The real destination can be a folder in current directory,
	// or a totally new directory defined by user, 
	// we have use expand address to find out.
	let real_dest = get_absolute_path(dest);
	// Turn to CString for ffi.
	let c_dest = CString::new(real_dest.as_bytes()).unwrap();
	// Change directory.
	match unsafe { chdir(c_dest.as_ptr()) } {
	// According to API, 0 means success, -1 fail.
		0 => { },
		_ => println!("CdError: failed to change directory."),
	}
}

fn safe_pipe(command: CommandLine,
			 mut history: &mut History){
	let args_cnt = command.args.len();
	let mut pipes: Vec<[i32; 2]> = Vec::new();
	let mut pids: Vec<i32>= Vec::new();
	// We only have command-1 pipes to set up
	for i in 0..args_cnt-1{
		pipes.push([0, 0]);
		unsafe {pipe(&mut pipes[i][0]);}
	}
	for i in 0..args_cnt{
		match unsafe{ fork() }{
			// Error.
			-1	=> { println!("ForkError: failed to fork."); },
			0	=> {
				unsafe {
				// Open a pipe in both ends.
					if i!= 0			{ dup2(pipes[i-1][0],0); }
					if i!=args_cnt-1	{ dup2(pipes[ i ][1],1); }
				}
				for j in 0..args_cnt-1{
					// Close each end of each pipe.
					unsafe{ close(pipes[j][0]); close(pipes[j][1]); }
				}
				// Don't worry, since this is a child process, 
				// pids is at my disposal, 
				// I can pop without worrying pushing it back.
				let sub_cmd = parse_cmd(command.args[i].clone());
				match sub_cmd.cmd.as_ref(){
					"history"	=> { io_redirection(&sub_cmd); print_history(&history) },
					"jobs"		=> { io_redirection(&sub_cmd); print_jobs(&mut history) },
					_			=> execute(&sub_cmd),
				};
				// This shouldn't run for external commands, 
				// it's reserved for internal command history and jobs.
				unsafe{ exit(0); }
			},
			pid => { pids.push(pid); },
		}
	}	
	// Only the parent gets here.
	for j in 0..args_cnt-1{
		// Close each end of each pipe.
		unsafe{ close(pipes[j][0]); close(pipes[j][1]); }
	}
	if command.if_continue{
		let mut job: Vec<String> = Vec::new();
		for sub_cmd in command.args.clone(){
			job.append(&mut parse_cmd(sub_cmd).args);
			job.push("|".to_string());
		}
		job.pop();
		history.jobs.push((pids, job));
	} else {
		let mut status: i32 = 1;
		for pid in pids {
			unsafe{ waitpid(pid, &mut status, 0); };
		}
	}
}

fn safe_execvp(args: Vec<String>){
	let cmd = args[0].clone();
	// Convert cmd(args[0]) and args into C style,
	let c_prog = CString::new(cmd.as_bytes()).unwrap();
	// The following lines must be wrong
	// multiple args couldn't be translated to C style.
	// For example, ls -a -l will only take -l pary.
	let c_args_temp: Vec<_> = args.iter()
			.map(|x| CString::new(x.as_bytes())
				.unwrap()).collect();
	let mut c_args: Vec<_> = c_args_temp.iter()
			.map(|x| x.as_ptr()).collect();
	c_args.push(std::ptr::null());
	match unsafe{ execvp(c_prog.as_ptr(), c_args.as_ptr()) }{
		-1 => println!("ExecError: failed to execute."),
		// Should execv(*const i8, *const *const i8) work properly, 
		// the following line won't print.
		_  => println!("ExecError: your computer's down 
						cause something crazy thing happened"),
	};
	unsafe{ exit(0); }
}

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
						get_absolute_path(file).as_bytes()
					).unwrap();
				let file_ptr = file.as_ptr();
				match arg.chars().nth(0).unwrap(){
					'<'	=> { 
						in_handle = unsafe { open(file_ptr, O_RDONLY) }; 
						if in_handle == -1 {println!("IORedirectionError: file to be opened not exist." );}
					},
					'>' => { 
						// In case such file don't exist, we create it by fopen.
						create_file(file_ptr);
						out_handle = unsafe { open(file_ptr, O_WRONLY) }; 
					},
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

fn execute(command: &CommandLine){
	let args = 	io_redirection(&command);
	safe_execvp(args);
}

fn external_cmd(command: CommandLine, mut history: &mut History){
	match unsafe { fork() } {
		// Error.
		-1	=> { println!("ForkError: failed to fork."); },
		// Child
		0 	=> execute(&command),
		// Parent.
		pid	=> {
			if command.if_continue {
				// The existence of '&'
				history.jobs.push((vec![pid], command.args));
			} else {
				let mut status: i32 = 1;
				unsafe{ waitpid(pid, &mut status, 0); };
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
			Ok(0)	=> { break; },
			Ok(_)	=> { },
			Err(_)	=> { println!("ReadLineError: failed to read."); continue; },
		}
		history.hist.push(cmd_line.clone());
		let command = parse_cmd(cmd_line);
		execute_cmd(command, &mut history);
	}
}

fn execute_cmd(command: CommandLine, mut history: &mut History){
	match command.cmd.as_ref() {
		""			=> return,
		"exit"		=> unsafe{ exit(0); },
		"pwd"		=> println!("{}", get_directory()),
		"cd"		=> { 
			match command.args.len() {
				1 => println!("CdError: no directory given."),
				// Real bash won't care if there are more parameters. 
				_ => safe_chdir(command.args[1].clone()),
			}
		},
		"pipe"		=> safe_pipe(command, &mut history),
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
	let open_mode = CString::new("w").unwrap();
	let open_file = fopen(file, open_mode.as_ptr()); 
	fclose(open_file);	
} }

fn get_directory() -> String{
	env::current_dir().unwrap().display().to_string()
}

fn get_absolute_path(dest: String) -> String {
	match dest.chars().nth(0).unwrap(){
		// Commands starting with a "/", whose ascii is 47,
		// in which case, dest is the real dest.
		'/'	=> dest,
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
	let mut status: i32 = 1;
	let mut new_jobs: Vec<(Vec<i32>, Vec<String>)> = Vec::new();
	for &(ref pids, ref job) in &history.jobs{
		let mut finished = true;
		for &pid in pids{
			if unsafe{ waitpid(pid, &mut status, libc::WNOHANG) }  == 0{
				finished = false;
				break;
			}
		}
		if !finished {
			print_job(&job); 
			new_jobs.push((pids.clone(), job.clone()));
		}
	}
	history.jobs = new_jobs;
}

fn print_history(history: &History){
	for i in 0..history.hist.len()-1{
		print!("{:5}  {}", i+1, history.hist[i]);
	}
}
