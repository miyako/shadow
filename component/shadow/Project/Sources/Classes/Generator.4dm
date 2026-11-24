// Generator.4dm — Public class: Shadow.Generator
// Provides API for generating image shadows via the shadow-cli binary
//
// Usage:
//   $shadow := cs.Shadow.Generator.new()
//   $result := $shadow.generate($inputFile; $outputFile; New object("type"; "drop"; "blur"; 10.0))

Class extends Object

// Class storage
property _cli : cs._CLI
property _process : cs._Process

// Constructor
Class constructor
	
	// Initialize internal classes
	This._cli := cs._CLI.new()
	This._process := cs._Process.new()

// Method: version() → Object
// Returns: {ok: True, value: {protocol, version}}
Function version : Object
	
	var $result : Object
	var $cliPath : Text
	var $procResult : Object
	var $json : Object
	
	// Check CLI availability
	$cliPath := This._cli.getBinaryPath()
	If ($cliPath="")
		return New object("ok"; False; "error"; New object("code"; "E_IO"; "message"; "shadow-cli binary not found"))
	End if 
	
	// Execute --version
	$procResult := This._process.executeSimple($cliPath; New collection("--version"))
	
	If ($procResult.exitCode#0)
		return New object("ok"; False; "error"; New object("code"; "E_RUNTIME"; "message"; $procResult.responseError))
	End if 
	
	// Parse JSON response
	$json := JSON Parse($procResult.response)
	
	return New object("ok"; True; "value"; $json)

// Method: capabilities() → Object
// Returns: {ok: True, value: {protocol, capabilities}}
Function capabilities : Object
	
	var $result : Object
	var $cliPath : Text
	var $procResult : Object
	var $json : Object
	
	$cliPath := This._cli.getBinaryPath()
	If ($cliPath="")
		return New object("ok"; False; "error"; New object("code"; "E_IO"; "message"; "shadow-cli binary not found"))
	End if 
	
	$procResult := This._process.executeSimple($cliPath; New collection("capabilities"))
	
	If ($procResult.exitCode#0)
		return New object("ok"; False; "error"; New object("code"; "E_RUNTIME"; "message"; $procResult.responseError))
	End if 
	
	$json := JSON Parse($procResult.response)
	
	return New object("ok"; True; "value"; $json)

// Method: generate($input; $output; $options) → Object
// $input: 4D.File object (file path mode) or Blob (stdin mode)
// $output: 4D.File object
// $options: Object with shadow parameters
// Returns: {ok: True, value: {width, height, output}} or {ok: False, error: {code, message}}
Function generate($input : Variant; $output : Variant; $options : Object) : Object
	
	return This._runCommand("generate"; $input; $output; $options; False)

// Method: shadowOnly($input; $output; $options) → Object
// Same as generate but outputs only the shadow layer
Function shadowOnly($input : Variant; $output : Variant; $options : Object) : Object
	
	return This._runCommand("generate"; $input; $output; $options; True)

// Method: _runCommand(command; input; output; options; shadowOnly) → Object
// Internal: build args and execute
Function _runCommand($command : Text; $input : Variant; $output : Variant; $options : Object; $shadowOnly : Boolean) : Object
	
	var $cliPath : Text
	var $args : Collection
	var $procResult : Object
	var $json : Object
	var $outputPath : Text
	var $inputPath : Text
	var $stdinBlob : Blob
	var $useStdin : Boolean
	var $useStdout : Boolean
	
	// Validate options
	If ($options=Null)
		$options := New object
	End if 
	
	// Check CLI availability
	$cliPath := This._cli.getBinaryPath()
	If ($cliPath="")
		return New object("ok"; False; "error"; New object("code"; "E_IO"; "message"; "shadow-cli binary not found"))
	End if 
	
	// Determine input mode
	$useStdin := False
	$stdinBlob := Null
	
	If (Value type($input)=Is blob)
		// Blob mode — feed via stdin
		$useStdin := True
		$stdinBlob := $input
		$inputPath := "-"
	Else 
		// File mode — validate it's a File object
		If (Value type($input)=Is object)
			Try 
				$inputPath := $input.path
			Catch 
				return New object("ok"; False; "error"; New object("code"; "E_ARGS"; "message"; "input must be a 4D.File or Blob"))
			End try 
		Else 
			return New object("ok"; False; "error"; New object("code"; "E_ARGS"; "message"; "input must be a 4D.File or Blob"))
		End if 
	End if 
	
	// Determine output mode
	If (Value type($output)=Is object)
		Try 
			$outputPath := $output.path
		Catch 
			return New object("ok"; False; "error"; New object("code"; "E_ARGS"; "message"; "output must be a 4D.File"))
		End try 
	Else 
		return New object("ok"; False; "error"; New object("code"; "E_ARGS"; "message"; "output must be a 4D.File"))
	End if 
	
	// Build arguments collection
	$args := New collection($command)
	
	// --input
	$args.push("--input")
	$args.push($inputPath)
	
	// --output
	$args.push("--output")
	$args.push($outputPath)
	
	// --type
	$args.push("--type")
	$args.push(This._getOption($options; "type"; "drop"))
	
	// --opacity
	$args.push("--opacity")
	$args.push(String(This._getOption($options; "opacity"; 0.5)))
	
	// --offset-x
	$args.push("--offset-x")
	$args.push(String(This._getOption($options; "offset-x"; 5)))
	
	// --offset-y
	$args.push("--offset-y")
	$args.push(String(This._getOption($options; "offset-y"; 5)))
	
	// --blur
	$args.push("--blur")
	$args.push(String(This._getOption($options; "blur"; 10.0)))
	
	// --color
	$args.push("--color")
	$args.push(This._getOption($options; "color"; "#000000"))
	
	// --scale
	$args.push("--scale")
	$args.push(String(This._getOption($options; "scale"; 1.0)))
	
	// --angle
	$args.push("--angle")
	$args.push(String(This._getOption($options; "angle"; 45)))
	
	// --length (optional)
	If ($options.length#Undefined)
		$args.push("--length")
		$args.push(String($options.length))
	End if 
	
	// --fade
	$args.push("--fade")
	$args.push(This._getOption($options; "fade"; "linear"))
	
	// --edge
	$args.push("--edge")
	$args.push(This._getOption($options; "edge"; "bottom"))
	
	// --curve-depth
	$args.push("--curve-depth")
	$args.push(String(This._getOption($options; "curve-depth"; 20.0)))
	
	// --light-x
	$args.push("--light-x")
	$args.push(String(This._getOption($options; "light-x"; 0.0)))
	
	// --light-y
	$args.push("--light-y")
	$args.push(String(This._getOption($options; "light-y"; 0.0)))
	
	// --light-z
	$args.push("--light-z")
	$args.push(String(This._getOption($options; "light-z"; 100.0)))
	
	// --canvas-expand
	$args.push("--canvas-expand")
	$args.push(String(This._getOption($options; "canvas-expand"; 20)))
	
	// --format
	$args.push("--format")
	$args.push(This._getOption($options; "format"; "png"))
	
	// --background
	$args.push("--background")
	$args.push(This._getOption($options; "background"; "transparent"))
	
	// --shadow-only flag
	If ($shadowOnly)
		$args.push("--shadow-only")
	End if 
	
	// Execute
	If ($useStdin)
		$procResult := This._process.execute($cliPath; $args; $stdinBlob)
	Else 
		$procResult := This._process.executeSimple($cliPath; $args)
	End if 
	
	// Parse result
	If ($procResult.exitCode=0)
		// Success — parse JSON response
		$json := JSON Parse($procResult.response)
		return New object("ok"; True; "value"; $json)
	Else 
		// Error — parse stderr JSON
		Try 
			$json := JSON Parse($procResult.responseError)
		Catch 
			$json := New object("protocol"; 1; "error"; New object("code"; "E_RUNTIME"; "message"; $procResult.responseError))
		End try 
		return New object("ok"; False; "error"; $json.error)
	End if 

// Method: _getOption(options; key; default) → Variant
// Helper: safely get an option value with a default
Function _getOption($options : Object; $key : Text; $default : Variant) : Variant
	
	If ($options[$key]#Undefined)
		return $options[$key]
	Else 
		return $default
	End if 
