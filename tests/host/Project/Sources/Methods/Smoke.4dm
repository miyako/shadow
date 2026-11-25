// Smoke.4dm — Host smoke test for Shadow.Generator
// Verifies the component namespace and API methods are accessible

var $shadow : cs.Shadow.Generator
var $result : Object
var $allPassed : Boolean
var $inputFile : 4D.File
var $outputFile : 4D.File

$allPassed := True

// Test 1: Constructor
$shadow := cs.Shadow.Generator.new()
ASSERT($shadow#Null; "Generator.new() returned Null")
print("  [PASS] Generator.new()")

// Test 2: version()
$result := $shadow.version()
If ($result.ok)
	ASSERT($result.value.protocol=1; "version() protocol should be 1")
	ASSERT($result.value.version="0.1.0"; "version() should return 0.1.0")
	print("  [PASS] version(): protocol=" + String($result.value.protocol) + " version=" + $result.value.version)
Else 
	$allPassed := False
	print("  [FAIL] version(): " + $result.error.message)
End if 

// Test 3: capabilities()
$result := $shadow.capabilities()
If ($result.ok)
	ASSERT($result.value.capabilities.types.length=6; "capabilities should list 6 types")
	ASSERT($result.value.capabilities.types.contains("drop"); "capabilities should include drop")
	ASSERT($result.value.capabilities.types.contains("inner"); "capabilities should include inner")
	print("  [PASS] capabilities(): " + String($result.value.capabilities.types.length) + " types")
Else 
	$allPassed := False
	print("  [FAIL] capabilities(): " + $result.error.message)
End if 

// Test 4: generate() — drop shadow
If (Folder("/RESOURCES/bin/").exists)
	$inputFile := File("/DATA/input_alpha_128.png")
	$outputFile := File("/DATA/output_drop.png")
	
	If ($inputFile.exists)
		$result := $shadow.generate($inputFile; $outputFile; New object("type"; "drop"; "blur"; 10.0; "opacity"; 0.5))
		If ($result.ok)
			ASSERT($result.value.ok=True; "generate() value.ok should be True")
			ASSERT($result.value.width>0; "generate() width should be > 0")
			ASSERT($result.value.height>0; "generate() height should be > 0")
			ASSERT($outputFile.exists; "output file should exist")
			print("  [PASS] generate(drop): " + String($result.value.width) + "x" + String($result.value.height))
		Else 
			$allPassed := False
			print("  [FAIL] generate(drop): " + $result.error.message)
		End if 
	Else 
		print("  [SKIP] generate(drop): input file not found at /DATA/input_alpha_128.png")
	End if 
	
	// Test 5: generate() — each shadow type
	For each ($type; New collection("drop"; "contact"; "curved"; "long"; "perspective"; "inner"))
		$outputFile := File("/DATA/output_" + $type + ".png")
		$result := $shadow.generate($inputFile; $outputFile; New object("type"; $type))
		If ($result.ok)
			ASSERT($outputFile.exists; "output file should exist for " + $type)
			print("  [PASS] generate(" + $type + "): " + String($result.value.width) + "x" + String($result.value.height))
		Else 
			$allPassed := False
			print("  [FAIL] generate(" + $type + "): " + $result.error.message)
		End if 
	End for 
	
	// Test 6: shadowOnly()
	$outputFile := File("/DATA/output_shadow_only.png")
	$result := $shadow.shadowOnly($inputFile; $outputFile; New object("type"; "drop"))
	If ($result.ok)
		print("  [PASS] shadowOnly(drop)")
	Else 
		$allPassed := False
		print("  [FAIL] shadowOnly(drop): " + $result.error.message)
	End if 
	
	// Test 7: Blob/stdin mode
	$result := $shadow.generate($inputFile.getContent(); File("/DATA/output_blob.png"); New object("type"; "contact"))
	If ($result.ok)
		print("  [PASS] generate(blob stdin)")
	Else 
		$allPassed := False
		print("  [FAIL] generate(blob stdin): " + $result.error.message)
	End if 
	
	// Test 8: Error handling — invalid type
	$result := $shadow.generate($inputFile; File("/DATA/invalid.png"); New object("type"; "bogus"))
	ASSERT($result.ok=False; "invalid type should return ok=False")
	ASSERT($result.error.code="E_ARGS"; "invalid type should return E_ARGS")
	print("  [PASS] error handling: invalid type → E_ARGS")
	
	// Test 9: Error handling — missing input
	$result := $shadow.generate(File("/nonexistent.png"); File("/tmp/none.png"); New object("type"; "drop"))
	ASSERT($result.ok=False; "missing input should return ok=False")
	ASSERT($result.error.code="E_IO"; "missing input should return E_IO")
	print("  [PASS] error handling: missing file → E_IO")
Else 
	print("  [SKIP] CLI binary not found — skipping generation tests")
End if 

// Summary
If ($allPassed)
	print("")
	print("All smoke tests passed.")
Else 
	print("")
	print("SMOKE TESTS FAILED — see errors above")
End if 

// Helper print function
Function print($msg : Text)
	// Output to 4D message area / log
	TRACE($msg)
