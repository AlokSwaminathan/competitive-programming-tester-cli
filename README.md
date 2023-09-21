# Competive Programming Tester
CLI Tool for easy testing of competitive programming problems  
Makes it easy to run tests using a terminal command instead of having to reupload to a website every time  
My first Rust project, please give me any feedback at swaminathanalok@gmail.com  

Supports C, C++, Java, and Python, however, C, Java, and Python use the versions that come installed on your PC (this could change in the future if people want)  

At the moment it works with USACO Problems and allows you to download test cases with the problem link (Not the link to the test case download), or the problem ID.  
If you want to download other types of problems they have to be zipped, and directly extract to test cases that are in the same directory with different file endings, and matching names to show that test cases correspond. You can also just configure the test cases to match those specifications and add a test from a folder.   

### Future Plans
  &ensp;Ability to download sample cases on USACO, Codeforces, and ATCoder.  
  &ensp;Support for AtCoder cases.  
  &ensp;Support for submission of problems (Not during competitions).  
  &ensp;Ability to run a test once by downloading it in the run command then deleting it.   

Installation (requires [Rust](https://www.rust-lang.org/tools/install)):  
  &ensp;`cargo install competitive-programming-tester-cli`  
Installation without Cargo:  
  &ensp;Should be a release with executables for Windows, Linux, and Mac, but it is up to you to get them in the right directory so they can be a command.  

Might add an install script later  

## Features(Most of this information can be found by using --help):  

### `cp-tester add` - Installs tests  
**Adding tests:**  
  &ensp;All test cases should be in the same directory level, and have different extensions for input and output. For example, case 10 would be 10.in and 10.out.  
  &ensp;`--link` takes a link to a zip file that must extract directly to test cases  
  &ensp;`--folder` takes a path to a folder  
  &ensp;`--usaco-link` takes a link to a USACO problem(The problem page not the test data link)  
  &ensp;`--usaco-id` takes a USACO problem ID(cpid=ID at the end of the link)  
**Extensions (DONT USE A .):**  
  &ensp;`--input-extension` takes the input extension that will be used to find the test cases, and that will be used if the test requires file IO(Default: in)  
  &ensp;`--output-extension` takes the output extension that will be used to find the test cases, and that will be used if the test requires file IO(Default: out)  
**Naming:**  
  &ensp;Default name:  
    &ensp;&ensp;For `--link` it is the name of the zip file that is downloaded  
    &ensp;&ensp;For `--folder` it is the name of the folder  
    &ensp;&ensp;For `--usaco-link` and `--usaco-id` the name is formatted <problem_name>_<division>_<competition><year>, such as find_and_replace_silver_jan23  
  &ensp;`--name` takes a name that overrides the default name  
**IO:**  
  &ensp;A test stores 2 values, `input_io` and `output_io`, which can either be STDIN/STDOUT respectively, or be file names  
  &ensp;The default values for these fields is STDIN and STDOUT, unless you are downloading a USACO problem using the specific flags, in which case it will be inferred.  
  &ensp;*This does unfortunately mean that if the test data has different extensions than the input and output, you will have to modify the test data first, but this isn't something I have seen often  

### `cp-tester config` - Interaction with the config  
This is the default config file (Stored wherever dirs::config_dir()/cp-tester is):  
```
{
  "default_cpp_ver": 17,
  "unicode_output": false,
  "default_time_limit": 5000,
  "gcc_flags": {
    "-lm": "",
    "-O2": ""
  },
  "gpp_flags": {
    "-O2": "",
    "-lm": ""
  }
}
```
`print` Prints the config   
`print-default` Prints the default config  
`reset` Resets the config to default  

There are sub-commands to edit each value in the config, as well as some others.
`unicode-output` determines if the test results after running the test on a file will be "PASSED" and "FAILED" or "✅" and "❌".  


### `cp-tester list` - Lists tests  
`cp-tester list` lists all test names in alphabetical order 
`--show-io` to show IO data for the tests(Default: false)  
`cp-tester list test <test>` to list cases for a specific test.   
  &ensp;`--cases` to list certain cases (Comma separated)(Default: all)  
  &ensp;`--show-input` and `--show-output` do what you expect and are both false by default as for some tests they can be very large. 
  
### `cp-tester remove` - Removes tests   
`cp-tester remove <test_name>` removes the test with that name  
`--all` to remove all cases(Default: false)  

### `cp-tester rename` - Renames tests  
`cp-tester rename <old_name> <new_name>` Renames test "old_name" to "new_name"  

### `cp-tester run` - Run test on a file  
`cp-tester run <name> --file <file>`  Valid file extensions are .c, .java, .py, and .cpp   
`--cases` to specify cases to run (comma separated)(Default: all cases)  
`--show-input` to show input(Default: false)  
`--compare-output` to compare your program's output to the desired output(Default: false)  
`--cpp-ver` to specify C++ version, defaults to that in the config(Default: 17)  
`--timeout` timeout in milliseconds, defaults to that in the config(Default: 5000ms)  


### Example usage:  
You want to work on http://www.usaco.org/index.php?page=viewproblem2&cpid=991  
Download the test cases:  
`cp-tester add --usaco-id 991`  
Run them on a file:  
`cp-tester run loan_repayment_silver_jan2020 --file path_to_solution.cpp`  
Result is: 
```
Test Case 1: 1 milliseconds
PASSED
Test Case 2: 47 milliseconds
PASSED
Test Case 3: 39 milliseconds
PASSED
Test Case 4: 1 milliseconds
PASSED
Test Case 5: 1 milliseconds
PASSED
Test Case 6: 4 milliseconds
PASSED
Test Case 7: 2 milliseconds
PASSED
Test Case 8: 2 milliseconds
PASSED
Test Case 9: 2 milliseconds
PASSED
Test Case 10: 4 milliseconds
PASSED
Test Case 11: 13 milliseconds
PASSED
``` 
Passed all the cases are you can now move on!



