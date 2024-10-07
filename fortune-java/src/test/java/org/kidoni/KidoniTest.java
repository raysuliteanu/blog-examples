package org.kidoni;

import org.junit.jupiter.api.Test;

class KidoniTest {
    @Test
    void appHasAGreeting() {
        App classUnderTest = new App();
        assertNotNull(classUnderTest.getGreeting(), "app should have a greeting");
    }
}
