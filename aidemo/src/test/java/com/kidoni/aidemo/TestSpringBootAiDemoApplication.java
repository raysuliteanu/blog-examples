package com.kidoni.aidemo;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.test.context.TestConfiguration;
import org.springframework.boot.testcontainers.service.connection.ServiceConnection;
import org.springframework.context.annotation.Bean;
import org.testcontainers.containers.Neo4jContainer;
import org.testcontainers.containers.Neo4jLabsPlugin;

@TestConfiguration(proxyBeanMethods = false)
public class TestSpringBootAiDemoApplication {
    @Bean
    @ServiceConnection
    public Neo4jContainer<?> neo4jContainer() {
        return new Neo4jContainer<>("neo4j:5")
                .withLabsPlugins(Neo4jLabsPlugin.APOC);
    }

    public static void main(String[] args) {
        SpringApplication.from(SpringBootAiDemoApplication::main)
                .with(TestSpringBootAiDemoApplication.class)
                .run(args);
    }

}
