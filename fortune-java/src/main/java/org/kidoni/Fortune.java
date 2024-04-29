package org.kidoni;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.IOException;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.random.RandomGenerator;

public class Fortune {
    private static final String FORTUNE_DATA_HOME = "/usr/share/games/fortunes";

    public static void main(String[] args) throws IOException {
        final List<File> files = getFortuneFiles();

        Map<Integer, String> fortunes = new HashMap<>();
        final int[] count = {0};
        files.forEach(file -> {
            loadFortunesFromFile(file, fortunes, count);
        });

        generateFortune(count, fortunes);
    }

    private static List<File> getFortuneFiles() throws IOException {
        String env = System.getenv("FORTUNE_HOME");
        final String fortuneDataHome = env != null ? env : FORTUNE_DATA_HOME;

        final List<File> files = new ArrayList<>();

        try (DirectoryStream<Path> stream = Files.newDirectoryStream(Path.of(fortuneDataHome), "*.dat")) {
            stream.iterator().forEachRemaining(path -> {
                File datFile = path.toFile();
                File parent = datFile.getParentFile();
                File file = new File(parent, getFortuneFileName(datFile));
                files.add(file);
            });
        }
        return files;
    }

    private static String getFortuneFileName(File datFile) {
        return datFile.getName().substring(0, datFile.getName().lastIndexOf('.'));
    }

    private static void loadFortunesFromFile(File file, Map<Integer, String> fortunes, int[] count) {
        try (final BufferedReader reader = new BufferedReader(new FileReader(file))) {
            try {
                String line;
                while ((line = reader.readLine()) != null) {
                    StringBuilder builder = new StringBuilder();
                    while (line != null && !line.equals("%")) {
                        builder.append(line).append("\n");
                        line = reader.readLine();
                    }
                    fortunes.put(count[0], builder.toString());
                    ++count[0];
                }
            }
            catch (IOException e) {
                System.err.println("skipping a fortune: " + e.getMessage());
            }
        }
        catch (IOException e) {
            System.err.println("skipping file: " + e.getLocalizedMessage());
        }
    }

    private static void generateFortune(int[] count, Map<Integer, String> fortunes) {
        Random random = Random.from(RandomGenerator.getDefault());
        int i = random.nextInt(count[0]);
        System.out.println(fortunes.get(i));
    }
}
