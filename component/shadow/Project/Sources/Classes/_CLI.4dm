// _CLI.4dm — Internal class: resolve shadow-cli binary path
// Platform dispatch: macOS universal, Windows x64

Class extends Object

// Constructor — no initialization needed
Class constructor

// Method: getBinaryPath() → Text
// Resolves the path to the shadow-cli binary in /RESOURCES/bin/
Function getBinaryPath : Text
	
	var $path : Text
	var $binName : Text
	
	// Platform-specific binary name
	If (Is macOS)
		$binName := "shadow-cli"
	Else 
		$binName := "shadow-cli.exe"
	End if 
	
	// Resolve from /RESOURCES/bin/
	$path := File("/RESOURCES/bin/" + $binName).path
	
	If (File("/RESOURCES/bin/" + $binName).exists)
		return $path
	Else 
		// Fallback: check if binary is in the component's Resources
		$path := Get 4D folder(Current resources folder)+"bin"+Folder separator+$binName
		If (Test path name($path)=Is a document)
			return $path
		Else 
			// Not found — return empty, caller will handle error
			return ""
		End if 
	End if 
	
// Method: isAvailable() → Boolean
// Checks whether the CLI binary exists and is executable
Function isAvailable : Boolean
	
	var $path : Text
	$path := This.getBinaryPath()
	
	If ($path="")
		return False
	Else 
		return True
	End if 
