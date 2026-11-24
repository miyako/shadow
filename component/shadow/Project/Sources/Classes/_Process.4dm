// _Process.4dm — Internal class: SystemWorker execution wrapper
// Handles argument quoting, stdin/stdout, error capture

Class extends Object

// Constructor
Class constructor

// Method: quote(text) → Text
// Safely quote an argument for the command line (no shell interpretation)
Function quote($text : Text) : Text
	
	// Escape any embedded double quotes
	$text := Replace string($text; "\""; "\\\""; *)
	
	// Wrap in double quotes
	return "\""+$text+"\""
	
// Method: buildCommandLine($cliPath : Text; $args : Collection) : Text
// Build a safely quoted command line from the CLI path and arguments
Function buildCommandLine($cliPath : Text; $args : Collection) : Text
	
	var $cmd : Text
	var $i : Integer
	var $arg : Text
	
	// Quote the CLI path (may contain spaces)
	$cmd := This.quote($cliPath)
	
	// Append each quoted argument
	For ($i; 0; $args.length-1)
		$arg := String($args[$i])
		// Reject embedded NUL characters (security)
		If (Position(Char(0); $arg; *)>0)
			Throw (-1; "Embedded NUL character detected in argument")
		End if 
		$cmd := $cmd+" "+This.quote($arg)
	End for 
	
	return $cmd
	
// Method: execute($cliPath : Text; $args : Collection; $stdinBlob : Blob) : Object
// Execute the CLI with the given arguments, optionally feeding stdin blob
// Returns: {response, responseError, exitCode, errors}
Function execute($cliPath : Text; $args : Collection; $stdinBlob : Variant) : Object
	
	var $cmd : Text
	var $worker : 4D.SystemWorker
	var $options : Object
	var $result : Object
	
	// Build command line
	$cmd := This.buildCommandLine($cliPath; $args)
	
	// Configure SystemWorker options
	$options := New object
	$options.hideWindow := True
	$options.timeout := 30
	$options.encoding := "UTF-8"
	$options.responseType := "text"
	$options.errorType := "text"
	
	// Create worker
	$worker := 4D.SystemWorker.new($cmd; $options)
	
	// Feed stdin if blob provided
	If ($stdinBlob#Null)
		If (Value type($stdinBlob)=Is blob)
			$worker.postMessage($stdinBlob)
		End if 
	End if 
	
	// Close stdin
	$worker.closeInput()
	
	// Wait for completion
	$worker.wait()
	
	// Build result object
	$result := New object
	$result.response := $worker.response
	$result.responseError := $worker.responseError
	$result.exitCode := $worker.exitCode
	$result.errors := $worker.errors
	
	return $result
	
// Method: executeSimple($cliPath : Text; $args : Collection) : Object
// Execute without stdin blob (file-path mode only)
Function executeSimple($cliPath : Text; $args : Collection) : Object
	
	return This.execute($cliPath; $args; Null)
