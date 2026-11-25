// _CaptureError.4dm — Shared method: capture and structure error from try/catch
// Returns a standardized error object

var $1 : Object  // caught error from Catch block
var $result : Object

$result := New object
$result.code := "E_RUNTIME"
$result.message := "Unknown error"

If (Count parameters>0)
	If ($1.message#Null)
		$result.message := $1.message
	End if 
	If ($1.errCode#Null)
		$result.code := "E_" + String($1.errCode)
	End if 
End if 

return $result
