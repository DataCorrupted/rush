
Extra features:

Error Handling:
	overall:
		1. Upon each and every error occured, rush will handle it and keep moving without crashing.
		2. If unable to read, rush will report:
			"ReadLineError: failed to read."
	cd:
		1. If no arguments given, rush will report:
			"CdError: no directory given."
		2. If more than one arguments given, rush will ignore the rest.
		3. If path given is invalid, rush reports:
			"CdError: failed to change directory."
	kill:
		1. If no pid given, rush will report:
			"KillError: no pid given."
		2. If the pid is invalid or rush don't have the right the kill the pid, it reports:
			"KillError: failed to kill the process (pid:<pid number>)"
	external command:
		1. If fork failed, it reports:
			"ForkError: failed to fork."
		2. If I/O direction failed, it reports:
			"IORedirectionError: file to be opened not exist."
		3. external command may be invalid or useless:
			"ExecError: failed to execute."

Robust:
	1. Any amount \t, \ are allowed.
	2. When doing I/O redirection, filename can come after </> with or without \ , both works.
	3. When doing I/O and the the file doesn't exist:
		a) A file will be opened and cleared if it is opened to be write.
		b) A file won't open for read and rush will expect input from stdin.
