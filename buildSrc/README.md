# A Gradle plugin to separate integration tests from unit tests
Integration tests take longer than unit tests. By definition their scope is larger, and more things need to be included
in the tests, particularly in the test fixture setup. As such it is desirable to separate the execution of unit tests
from the integration tests, such as in a CI pipeline. In Gradle, this can be done by creating a new "source set" that
contains just the integration test code.

This example is taken from the 
[Gradle documentation](https://docs.gradle.org/current/samples/sample_jvm_multi_project_with_additional_test_types.html).
By convention, just name a directory `buildSrc` and place it in your Gradle project root directory.
```bash
myProject/
  buildSrc/
  src/
  build.gradle
  settings.gradle
```
Gradle will  automatically see and include this directory. In this case we're defining a plugin. To use it simply declare 
it in your project's `build.gradle` `plugins` ...
```Gradle
plugins {
	id 'java'

	id 'my-project.java-conventions'
}
```
__NOTE:__ The plugin name is whatever the name of the .gradle file name is in `buildSrc/src/main/groovy`. Name it
however you like.
