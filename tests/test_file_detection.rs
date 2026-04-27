use pulse::test_detection::is_test_file;

type Cases<'a> = &'a [(&'a str, bool, &'a str)];

fn assert_cases(cases: Cases) {
    let mut failures: Vec<String> = Vec::new();
    for (path, expected, desc) in cases {
        let actual = is_test_file(path);
        if actual != *expected {
            failures.push(format!(
                "  [{desc}] path={path:?} expected={expected} actual={actual}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

// ==========================================================================
// Python — pytest, unittest, Django, nose conventions + production lookalikes
// ==========================================================================
#[test]
fn python_test_files() {
    let cases: Cases = &[
        // pytest naming
        ("test_foo.py", true, "pytest test_*"),
        ("test_user_auth.py", true, "pytest multi-word"),
        ("test_.py", true, "minimal test_ prefix"),
        ("foo_test.py", true, "alt _test suffix"),
        ("user_auth_test.py", true, "multi-word _test suffix"),
        ("conftest.py", true, "pytest fixture file"),
        ("tests.py", true, "django tests.py"),
        // unittest dirs
        ("tests/foo.py", true, "tests/ at root"),
        ("/abs/path/tests/api.py", true, "absolute tests path"),
        ("./tests/api.py", true, "relative tests path"),
        ("src/tests/helpers.py", true, "nested tests dir"),
        ("project/test/foo.py", true, "test/ singular"),
        ("project/__tests__/foo.py", true, "__tests__ jest-like"),
        // production-level patterns
        ("api/users/tests/test_views.py", true, "django app tests"),
        ("backend/services/tests/test_billing.py", true, "deep tests"),
        ("integration_tests/test_pipeline.py", true, "test_ prefix wins regardless of dir"),
        ("tests/integration/test_db.py", true, "integration tests dir"),
        ("tests/unit/test_parser.py", true, "unit tests dir"),
        ("tests/conftest.py", true, "conftest in tests"),
        ("tests/__init__.py", true, "init in tests dir"),
        // negatives — production code with test-like substrings
        ("foo.py", false, "ordinary module"),
        ("models.py", false, "django models"),
        ("views.py", false, "django views"),
        ("attest.py", false, "starts with 'at' not 'test_'"),
        ("contestant.py", false, "contains test as substring"),
        ("pretest.py", false, "starts with 'pre'"),
        ("latest.py", false, "ends with 'test' lowercase, no boundary"),
        ("greatest.py", false, "ends with 'test' lowercase"),
        ("protested.py", false, "contains test substring"),
        ("contest.py", false, "ends with 'test' lowercase"),
        ("tester.py", false, "starts with test, not test_"),
        ("testing.py", false, "starts with test, not test_"),
        ("testify.py", false, "library import-name lookalike"),
        ("testimonials.py", false, "ends with no boundary"),
        ("testbed.py", false, "single word, no underscore"),
        ("testutils.py", false, "no separator after test"),
        ("test.py", false, "no underscore, single word"),
        ("manifest.py", false, "no test boundary"),
        ("requestor.py", false, "no boundary"),
        ("digest.py", false, "ends with 'est'"),
        ("forest.py", false, "ends with 'est'"),
        // nested production
        ("src/api/views.py", false, "deep production module"),
        ("backend/services/billing.py", false, "service module"),
        ("lib/utils/helpers.py", false, "utils module"),
        ("vendor/test/foo.py", true, "vendor test dir still matches"),
        // setup/build
        ("setup.py", false, "package setup"),
        ("manage.py", false, "django manage"),
        ("conftest_helpers.py", false, "named like conftest but not"),
        ("test_helpers/utils.py", false, "directory not named tests"),
        ("test_data.py", true, "test_ prefix"),
        ("data_test.py", true, "_test suffix"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// JavaScript / TypeScript — Jest, Vitest, Mocha, Cypress, Playwright
// ==========================================================================
#[test]
fn javascript_test_files() {
    let cases: Cases = &[
        // Jest/Vitest standard
        ("foo.test.js", true, "jest .test."),
        ("foo.spec.js", true, "mocha .spec."),
        ("foo.test.jsx", true, "jest jsx"),
        ("foo.test.mjs", true, "esm test"),
        ("foo.test.cjs", true, "cjs test"),
        ("user.spec.js", true, "spec naming"),
        ("MyComponent.test.js", true, "component test"),
        ("api/users.test.js", true, "nested test"),
        ("src/components/Button.test.js", true, "deep test"),
        // __tests__ directory
        ("__tests__/foo.js", true, "__tests__ root"),
        ("src/__tests__/api.js", true, "nested __tests__"),
        ("src/components/__tests__/Button.js", true, "deep __tests__"),
        // tests/ directory
        ("tests/foo.js", true, "tests dir"),
        ("test/api.js", true, "test dir"),
        // E2E/Cypress/Playwright
        ("foo.e2e.js", true, "e2e suffix"),
        ("login.cy.js", true, "cypress suffix"),
        ("checkout.e2e.js", true, "e2e flow"),
        // production negatives
        ("foo.js", false, "ordinary module"),
        ("index.js", false, "entry point"),
        ("app.js", false, "main app"),
        ("latest.js", false, "lowercase test substring"),
        ("manifest.js", false, "manifest"),
        ("contestant.js", false, "test substring"),
        ("attestation.js", false, "starts with at"),
        ("requestor.js", false, "no boundary"),
        ("digestor.js", false, "no boundary"),
        ("forester.js", false, "no test pattern"),
        ("Tester.js", false, "production class lookalike"),
        ("testify.js", false, "library name lookalike"),
        ("testbed.js", false, "single-word no separator"),
        // weird edge cases
        ("foo.testing.js", false, ".testing. not .test."),
        ("foo.tester.js", false, ".tester. not .test."),
        ("foo.testdata.js", false, ".testdata. not .test."),
        ("test.js", false, "no separator suffix"),
        ("spec.js", false, "no separator suffix"),
        ("specifications.js", false, "spec substring no boundary"),
        // config files often near tests
        ("jest.config.js", false, "jest config not test"),
        ("vitest.config.js", false, "vitest config"),
        ("cypress.config.js", false, "cypress config"),
        ("playwright.config.js", false, "playwright config"),
        // .test. and .spec. at deeper paths
        ("packages/core/src/foo.test.js", true, "monorepo test"),
        ("apps/web/components/Button.spec.jsx", true, "monorepo spec"),
        // capitalised dir Tests
        ("src/Tests/foo.js", true, "capitalised Tests dir"),
        // mixed
        ("foo.tests.js", true, ".tests. plural"),
        ("foo.specs.js", true, ".specs. plural"),
        // negative: substring within filename
        ("attest.js", false, "starts with at"),
        ("hottest.js", false, "ends with test lowercase"),
        ("smoketests.js", false, "no boundary"),
        ("foo.config.js", false, "config not test"),
        ("backbone.js", false, "library name"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// TypeScript — same patterns but .ts/.tsx files
// ==========================================================================
#[test]
fn typescript_test_files() {
    let cases: Cases = &[
        ("foo.test.ts", true, "ts test"),
        ("foo.test.tsx", true, "tsx test"),
        ("foo.spec.ts", true, "ts spec"),
        ("Component.test.tsx", true, "react ts test"),
        ("hooks.test.ts", true, "hooks test"),
        ("services/api.test.ts", true, "nested ts test"),
        ("__tests__/foo.ts", true, "__tests__ ts"),
        ("tests/api.ts", true, "tests dir ts"),
        ("test/foo.ts", true, "test dir ts"),
        ("Spec/foo.ts", true, "Spec dir ts"),
        ("foo.e2e.ts", true, "e2e ts"),
        ("login.cy.ts", true, "cypress ts"),
        ("packages/foo/src/bar.test.ts", true, "monorepo ts"),
        ("apps/api/src/users.spec.ts", true, "nested spec ts"),
        ("Foo.test.tsx", true, "PascalCase ts"),
        ("foo.tests.ts", true, "tests plural"),
        ("foo.specs.ts", true, "specs plural"),
        // negatives
        ("foo.ts", false, "ordinary ts module"),
        ("index.ts", false, "entry"),
        ("types.ts", false, "type defs"),
        ("App.tsx", false, "react root"),
        ("foo.d.ts", false, "type declaration"),
        ("latest.ts", false, "lowercase test"),
        ("manifest.ts", false, "manifest"),
        ("contestant.ts", false, "substring"),
        ("attestation.ts", false, "at-prefix"),
        ("Tester.ts", false, "production class"),
        ("testify.ts", false, "lib name"),
        ("testbed.ts", false, "single word"),
        ("test.ts", false, "no separator"),
        ("spec.ts", false, "no separator"),
        ("requestor.ts", false, "no boundary"),
        ("digestor.ts", false, "no boundary"),
        ("forester.ts", false, "no boundary"),
        ("Specifications.ts", false, "spec substring"),
        ("hottest.ts", false, "lowercase test"),
        ("greatest.ts", false, "lowercase test"),
        ("foo.testing.ts", false, ".testing. not .test."),
        ("foo.tester.ts", false, "tester not test"),
        ("foo.config.ts", false, "config"),
        ("vite.config.ts", false, "vite config"),
        ("vitest.config.ts", false, "vitest config"),
        ("playwright.config.ts", false, "playwright config"),
        ("attest.ts", false, "starts with at"),
        ("smoketests.ts", false, "no boundary"),
        ("hookstore.ts", false, "no test"),
        ("forge.ts", false, "no test"),
        ("digest.ts", false, "ends with est"),
        ("ingest.ts", false, "ends with est"),
        // dir-based wins regardless
        ("packages/foo/__tests__/Bar.ts", true, "monorepo __tests__"),
        ("apps/admin/tests/login.ts", true, "tests in monorepo"),
        ("services/billing/test/charge.ts", true, "test singular monorepo"),
        ("foo/Tests/bar.ts", true, "capitalised dir"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Rust — tests/ integration dir; inline #[cfg(test)] not file-detectable
// ==========================================================================
#[test]
fn rust_test_files() {
    let cases: Cases = &[
        // tests/ integration dir (universal)
        ("tests/integration.rs", true, "tests dir"),
        ("tests/api.rs", true, "tests/api"),
        ("tests/common/mod.rs", true, "tests/common nested"),
        ("crate/tests/foo.rs", true, "crate/tests"),
        ("/abs/repo/tests/lib_smoke.rs", true, "absolute"),
        ("./tests/parser.rs", true, "relative"),
        // suffix patterns
        ("foo_test.rs", true, "_test suffix"),
        ("parser_test.rs", true, "_test naming"),
        ("foo_tests.rs", true, "_tests plural"),
        ("foo_spec.rs", true, "_spec uncommon"),
        // negatives — production code
        ("foo.rs", false, "ordinary module"),
        ("lib.rs", false, "lib root"),
        ("main.rs", false, "main"),
        ("mod.rs", false, "module file"),
        ("src/parser.rs", false, "src module"),
        ("src/api/users.rs", false, "deep module"),
        ("latest.rs", false, "lowercase test substring"),
        ("manifest.rs", false, "manifest"),
        ("contestant.rs", false, "substring"),
        ("attestation.rs", false, "at-prefix"),
        ("forester.rs", false, "no boundary"),
        ("digester.rs", false, "no boundary"),
        ("tester.rs", false, "tester not test"),
        ("test.rs", false, "single word, no separator"),
        ("testing.rs", false, "starts with test, no underscore"),
        ("testify.rs", false, "lib name lookalike"),
        ("testbed.rs", false, "no separator"),
        ("requests.rs", false, "no boundary"),
        ("forest.rs", false, "ends with est"),
        ("digest.rs", false, "ends with est"),
        ("nest.rs", false, "ends with est"),
        ("greatest.rs", false, "lowercase test"),
        // Rust convention: inline tests in src files NOT detected by filename
        ("src/parser.rs", false, "inline cfg(test) module not detectable"),
        ("src/lib.rs", false, "lib with #[cfg(test)] not file-level"),
        // edge cases
        ("benches/bench.rs", false, "benchmark dir not test"),
        ("examples/demo.rs", false, "examples dir not test"),
        ("build.rs", false, "build script"),
        ("Cargo.toml", false, "manifest TOML — not Rust ext"),
        // weird paths
        ("crate/tests/sub/deep_test.rs", true, "deep in tests dir"),
        ("crate/src/tests.rs", false, "module-named tests not in dir"),
        ("crate/src/test.rs", false, "module-named test not in dir"),
        ("crate/src/test_utils.rs", false, "test_utils — production helper"),
        ("crate/src/testing.rs", false, "production testing utils"),
        ("workspace/foo/tests/bar.rs", true, "workspace tests"),
        ("workspace/foo/src/bar.rs", false, "workspace src"),
        // false-positive guards
        ("contest.rs", false, "ends with test lowercase"),
        ("hottest.rs", false, "lowercase"),
        ("smoketests.rs", false, "no boundary"),
        ("benchmarktest.rs", false, "no boundary"),
        ("integration.rs", false, "named integration but not in tests/"),
        ("./crate/tests/integration.rs", true, "relative tests/"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Go — _test.go convention
// ==========================================================================
#[test]
fn go_test_files() {
    let cases: Cases = &[
        ("foo_test.go", true, "go _test convention"),
        ("user_auth_test.go", true, "multi-word _test"),
        ("api_test.go", true, "api _test"),
        ("handler_test.go", true, "handler test"),
        ("internal/auth/login_test.go", true, "nested test"),
        ("pkg/user/store_test.go", true, "pkg test"),
        ("/abs/path/foo_test.go", true, "absolute"),
        ("./cmd/server/main_test.go", true, "relative"),
        // tests/ dir (uncommon but possible)
        ("tests/integration.go", true, "tests dir"),
        ("test/foo.go", true, "test dir"),
        // negatives
        ("foo.go", false, "ordinary module"),
        ("main.go", false, "main"),
        ("server.go", false, "server"),
        ("handler.go", false, "handler"),
        ("user_test_helpers.go", false, "helpers, not _test.go"),
        ("test_helpers.go", false, "starts with test_ — not Go convention"),
        ("testdata.go", false, "single word"),
        ("latest.go", false, "lowercase test substring"),
        ("manifest.go", false, "manifest"),
        ("attest.go", false, "starts with at"),
        ("contestant.go", false, "test substring"),
        ("forester.go", false, "no boundary"),
        ("digester.go", false, "no boundary"),
        ("requestor.go", false, "no boundary"),
        ("Tester.go", false, "production"),
        ("testbed.go", false, "no separator"),
        ("test.go", false, "single word"),
        ("contest.go", false, "ends with test lowercase"),
        ("hottest.go", false, "lowercase"),
        ("greatest.go", false, "lowercase"),
        ("digest.go", false, "ends est"),
        ("forest.go", false, "ends est"),
        ("ingest.go", false, "ends est"),
        ("nest.go", false, "ends est"),
        ("requests.go", false, "no boundary"),
        ("conftest.go", false, "Python convention, not Go"),
        ("tests.go", false, "module named tests, not in /tests/"),
        // edge cases — multi-package Go
        ("cmd/api/main_test.go", true, "cmd test"),
        ("internal/storage/db_test.go", true, "internal test"),
        ("pkg/lib/helpers_test.go", true, "pkg helpers test"),
        ("vendor/foo/bar_test.go", true, "vendor test (filename match)"),
        // benchmarks live alongside tests in Go
        ("bench_test.go", true, "go benchmark naming"),
        ("foo_bench.go", false, "no _test suffix"),
        // example tests
        ("example_test.go", true, "go example tests"),
        // negatives
        ("test_data.go", false, "test_ prefix not Go"),
        ("testing_helpers.go", false, "helpers production"),
        ("tester.go", false, "tester not test"),
        ("attestation.go", false, "at-prefix"),
        ("smoketests.go", false, "no boundary"),
        ("integration.go", false, "no test suffix"),
        ("handler_testdata.go", false, "ends in testdata not test"),
        ("foo_t.go", false, "abbreviated, not _test"),
        ("./tests/integration.go", true, "relative tests dir"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Java — *Test.java, *Tests.java, *IT.java conventions
// ==========================================================================
#[test]
fn java_test_files() {
    let cases: Cases = &[
        // standard suffixes
        ("FooTest.java", true, "Test suffix"),
        ("FooTests.java", true, "Tests plural"),
        ("UserServiceTest.java", true, "service test"),
        ("BillingControllerTest.java", true, "controller test"),
        ("RepositoryTests.java", true, "repository tests"),
        ("Test.java", true, "just Test"),
        ("Tests.java", true, "just Tests"),
        ("FooSpec.java", true, "Spec suffix"),
        ("FooSpecs.java", true, "Specs plural"),
        // src/test/java/ dir
        ("src/test/java/com/foo/BarTest.java", true, "maven test dir"),
        ("src/test/java/Foo.java", true, "maven test no suffix"),
        ("project/src/test/java/Util.java", true, "deep maven test"),
        // tests/ dir
        ("tests/Foo.java", true, "tests dir"),
        ("project/tests/IntegrationTest.java", true, "nested tests"),
        // negatives — production lookalikes
        ("Foo.java", false, "ordinary class"),
        ("UserService.java", false, "service"),
        ("Controller.java", false, "controller"),
        ("Latest.java", false, "Latest — capital L only"),
        ("LATEST.java", false, "uppercase LATEST not Test suffix"),
        ("Greatest.java", false, "lowercase test"),
        ("Contests.java", false, "single word lowercase"),
        ("Contest.java", false, "single word"),
        ("Tester.java", false, "Tester ends in er"),
        ("Testing.java", false, "Testing — no Test suffix boundary"),
        ("Testify.java", false, "lib lookalike"),
        ("TestBed.java", false, "Test prefix only, no suffix"),
        ("TestUtils.java", false, "Test prefix utility"),
        ("TestHelpers.java", false, "Test prefix helper"),
        ("Manifest.java", false, "manifest"),
        ("Forest.java", false, "ends with est"),
        ("Digest.java", false, "ends with est"),
        ("Request.java", false, "ends with est"),
        ("Attestation.java", false, "at-prefix"),
        // tricky edge cases
        ("RequestTest.java", true, "Request + Test — valid test class"),
        ("LatestTest.java", true, "Latest + Test"),
        ("LatestTests.java", true, "Latest + Tests"),
        ("RequestSpec.java", true, "Request + Spec"),
        ("FooBarTest.java", true, "compound class test"),
        ("FooBarTests.java", true, "compound tests"),
        ("XMLTest.java", true, "acronym + Test"),
        ("XMLTests.java", true, "acronym + Tests"),
        ("HTTPTest.java", true, "HTTP test"),
        // integration test conventions (less universal)
        ("FooIT.java", false, "IT suffix not in our rules"),
        ("FooITCase.java", false, "ITCase not in our rules"),
        // Spock
        ("UserServiceSpec.java", true, "Spec convention"),
        // package-info
        ("package-info.java", false, "package info"),
        ("module-info.java", false, "module info"),
        // weird lookalikes
        ("AssertTest.java", true, "Assert + Test"),
        ("Asserter.java", false, "Asserter not test"),
        ("Inspector.java", false, "production"),
        ("ProtestSong.java", false, "no Test suffix"),
        ("TestProtester.java", false, "no Test suffix at end"),
        ("FooTestData.java", false, "TestData suffix not Test"),
        ("FooTestUtil.java", false, "TestUtil suffix not Test"),
        ("FooTestable.java", false, "Testable interface, not Test"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Kotlin — *Test.kt, *Tests.kt, *Spec.kt + src/test/kotlin/
// ==========================================================================
#[test]
fn kotlin_test_files() {
    let cases: Cases = &[
        ("FooTest.kt", true, "Kotlin test"),
        ("FooTests.kt", true, "Tests plural"),
        ("UserRepositoryTest.kt", true, "repository test"),
        ("FooKtTest.kt", true, "FooKt + Test"),
        ("FooSpec.kt", true, "Kotest spec"),
        ("FooSpecs.kt", true, "specs"),
        ("Test.kt", true, "Test alone"),
        ("Tests.kt", true, "Tests alone"),
        ("Foo.test.kts", true, ".test. script"),
        // src/test/kotlin/
        ("src/test/kotlin/com/foo/BarTest.kt", true, "gradle test dir"),
        ("src/test/kotlin/Util.kt", true, "test no suffix"),
        ("project/src/test/kotlin/Foo.kt", true, "deep gradle test"),
        ("tests/Foo.kt", true, "tests dir"),
        // negatives
        ("Foo.kt", false, "production"),
        ("Service.kt", false, "service"),
        ("Latest.kt", false, "lowercase test"),
        ("Contests.kt", false, "single word"),
        ("Tester.kt", false, "ends in er"),
        ("Testing.kt", false, "no boundary"),
        ("TestBed.kt", false, "Test prefix only"),
        ("TestUtils.kt", false, "Test prefix utility"),
        ("Testify.kt", false, "lookalike"),
        ("Manifest.kt", false, "manifest"),
        ("Digest.kt", false, "ends est"),
        ("Forest.kt", false, "ends est"),
        ("Request.kt", false, "ends est"),
        ("Attestation.kt", false, "at-prefix"),
        ("Greatest.kt", false, "lowercase test"),
        ("Contest.kt", false, "single word"),
        // valid test-ish
        ("RequestTest.kt", true, "Request + Test"),
        ("LatestTest.kt", true, "Latest + Test"),
        ("XMLTest.kt", true, "acronym + Test"),
        ("HTTPTest.kt", true, "HTTP test"),
        ("APIServiceTest.kt", true, "API service test"),
        ("DataClassTests.kt", true, "data class tests"),
        ("AssertTest.kt", true, "assert + test"),
        // dialect / scripts
        ("build.gradle.kts", false, "gradle script"),
        ("settings.gradle.kts", false, "settings"),
        // negatives extra
        ("FooTestData.kt", false, "TestData"),
        ("FooTestUtil.kt", false, "TestUtil"),
        ("FooTestable.kt", false, "Testable"),
        ("AbstractTester.kt", false, "Tester"),
        ("FooTesting.kt", false, "Testing"),
        ("BookContest.kt", false, "Contest"),
        ("ProtestSong.kt", false, "Protest"),
        ("Inspector.kt", false, "Inspector"),
        ("ListItem.kt", false, "no test"),
        ("MainActivity.kt", false, "android activity"),
        ("MyFragment.kt", false, "android fragment"),
        ("Foo.kts", false, "kotlin script not test"),
        ("FooSpecial.kt", false, "ends in 'cial' not Spec"),
        // path-only triggers
        ("project/src/main/kotlin/Foo.kt", false, "main kotlin not test"),
        ("project/src/test/kotlin/sub/Bar.kt", true, "deep test kotlin"),
        ("./src/test/kotlin/Foo.kt", true, "relative gradle test"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Groovy — Spock and JUnit conventions
// ==========================================================================
#[test]
fn groovy_test_files() {
    let cases: Cases = &[
        ("FooTest.groovy", true, "JUnit groovy"),
        ("FooTests.groovy", true, "tests plural"),
        ("FooSpec.groovy", true, "Spock"),
        ("FooSpecs.groovy", true, "specs plural"),
        ("Test.groovy", true, "just Test"),
        ("UserServiceSpec.groovy", true, "Spock service"),
        ("BillingControllerTest.groovy", true, "controller test"),
        ("src/test/groovy/com/Foo.groovy", true, "gradle test dir"),
        ("tests/Foo.groovy", true, "tests dir"),
        ("project/src/test/groovy/Bar.groovy", true, "gradle nested"),
        ("FooSpec.groovy", true, "spec spock"),
        // negatives
        ("Foo.groovy", false, "ordinary"),
        ("BuildScript.groovy", false, "build"),
        ("Latest.groovy", false, "lowercase test"),
        ("Contests.groovy", false, "single word"),
        ("Tester.groovy", false, "tester"),
        ("Testing.groovy", false, "testing"),
        ("Testify.groovy", false, "lib name"),
        ("TestUtils.groovy", false, "Test prefix only"),
        ("Manifest.groovy", false, "manifest"),
        ("Digest.groovy", false, "ends est"),
        ("Forest.groovy", false, "ends est"),
        ("Request.groovy", false, "ends est"),
        ("Attestation.groovy", false, "at-prefix"),
        ("RequestTest.groovy", true, "request test"),
        ("RequestSpec.groovy", true, "request spec"),
        ("XMLTest.groovy", true, "acronym test"),
        ("FooSpecial.groovy", false, "ends cial"),
        ("MyContest.groovy", false, "contest"),
        ("ProtestSong.groovy", false, "protest"),
        ("Greatest.groovy", false, "lowercase test"),
        ("FooTestData.groovy", false, "TestData"),
        ("FooTestUtil.groovy", false, "TestUtil"),
        ("FooTestable.groovy", false, "Testable"),
        ("FooTesting.groovy", false, "Testing"),
        ("Inspector.groovy", false, "production"),
        ("MyClass.groovy", false, "production"),
        ("BaseSpec.groovy", true, "BaseSpec test"),
        ("BaseTest.groovy", true, "BaseTest test"),
        ("AbstractTester.groovy", false, "Tester"),
        ("DataClassTests.groovy", true, "data class tests"),
        ("AssertTest.groovy", true, "assert test"),
        ("Asserter.groovy", false, "asserter"),
        ("HTTPTests.groovy", true, "http tests"),
        ("APITest.groovy", true, "api test"),
        ("foo.spec.groovy", true, ".spec. naming"),
        ("foo.test.groovy", true, ".test. naming"),
        ("foo_test.groovy", true, "_test naming"),
        ("foo_spec.groovy", true, "_spec naming"),
        ("project/src/main/groovy/Foo.groovy", false, "main groovy"),
        ("project/src/test/groovy/sub/Bar.groovy", true, "deep test"),
        ("./src/test/groovy/Foo.groovy", true, "relative test groovy"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// C# — *Test.cs, *Tests.cs, *Spec.cs, *Specs.cs
// ==========================================================================
#[test]
fn csharp_test_files() {
    let cases: Cases = &[
        ("FooTest.cs", true, "test class"),
        ("FooTests.cs", true, "tests plural"),
        ("UserServiceTests.cs", true, "service tests"),
        ("BillingControllerTest.cs", true, "controller test"),
        ("FooSpec.cs", true, "spec"),
        ("FooSpecs.cs", true, "specs"),
        ("Test.cs", true, "just Test"),
        ("Tests.cs", true, "just Tests"),
        ("tests/Foo.cs", true, "tests dir"),
        ("Tests/Foo.cs", true, "Tests dir"),
        ("Project.Tests/Foo.cs", false, "dotted project name not /Tests/"),
        ("/repo/Project.Tests/UserService.cs", false, "csproj naming, file not in tests dir"),
        // negatives
        ("Foo.cs", false, "production"),
        ("UserService.cs", false, "service"),
        ("Controller.cs", false, "controller"),
        ("Latest.cs", false, "lowercase test"),
        ("Contests.cs", false, "single word"),
        ("Tester.cs", false, "ends er"),
        ("Testing.cs", false, "no suffix boundary"),
        ("Testify.cs", false, "lib name"),
        ("TestBed.cs", false, "Test prefix only"),
        ("TestUtils.cs", false, "Test prefix util"),
        ("TestHelpers.cs", false, "Test prefix helper"),
        ("Manifest.cs", false, "manifest"),
        ("Digest.cs", false, "ends est"),
        ("Forest.cs", false, "ends est"),
        ("Request.cs", false, "ends est"),
        ("Attestation.cs", false, "at-prefix"),
        ("Greatest.cs", false, "lowercase test"),
        ("Contest.cs", false, "single word"),
        ("RequestTest.cs", true, "request+test"),
        ("LatestTests.cs", true, "latest+tests"),
        ("XMLTest.cs", true, "acronym test"),
        ("HTTPTest.cs", true, "http test"),
        ("APIServiceTests.cs", true, "api service tests"),
        ("AssertTest.cs", true, "assert test"),
        ("Asserter.cs", false, "asserter"),
        ("FooTestData.cs", false, "TestData suffix"),
        ("FooTestUtil.cs", false, "TestUtil suffix"),
        ("FooTestable.cs", false, "Testable suffix"),
        ("FooTesting.cs", false, "Testing suffix"),
        ("ProtestSong.cs", false, "no test suffix"),
        ("MyContest.cs", false, "Contest"),
        ("Inspector.cs", false, "production"),
        ("Program.cs", false, "entry"),
        ("Startup.cs", false, "asp.net startup"),
        ("AssemblyInfo.cs", false, "assembly info"),
        ("Foo.Designer.cs", false, "designer file"),
        ("Foo.cshtml.cs", false, "razor codebehind"),
        // dirs again
        ("MyApp.UnitTests/Foo.cs", false, "csproj name not dir"),
        ("MyApp.UnitTests/tests/Foo.cs", true, "nested tests"),
        ("project/Tests/Bar.cs", true, "Tests dir"),
        ("project/test/Bar.cs", true, "test dir"),
        ("DataClassTests.cs", true, "data class tests"),
        ("BaseTest.cs", true, "base test"),
        ("BaseSpec.cs", true, "base spec"),
        ("HTTPTests.cs", true, "http tests"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Swift — *Tests.swift, *Test.swift, *Spec.swift + Tests/ dir (XCTest)
// ==========================================================================
#[test]
fn swift_test_files() {
    let cases: Cases = &[
        ("FooTests.swift", true, "XCTest plural"),
        ("FooTest.swift", true, "Test singular"),
        ("UserServiceTests.swift", true, "service tests"),
        ("FooSpec.swift", true, "spec"),
        ("FooSpecs.swift", true, "specs"),
        ("Tests.swift", true, "just Tests"),
        ("Test.swift", true, "just Test"),
        // Tests/ directory (XCTest convention)
        ("Tests/Foo.swift", true, "Tests dir"),
        ("MyAppTests/Foo.swift", false, "MyAppTests dir name not /Tests/"),
        ("Sources/Foo.swift", false, "Sources dir"),
        ("project/Tests/MyAppTests/Foo.swift", true, "nested Tests"),
        ("./Tests/Foo.swift", true, "relative Tests"),
        // negatives
        ("Foo.swift", false, "production"),
        ("ContentView.swift", false, "swiftui"),
        ("AppDelegate.swift", false, "uikit"),
        ("Latest.swift", false, "lowercase test"),
        ("Contests.swift", false, "single word"),
        ("Tester.swift", false, "ends er"),
        ("Testing.swift", false, "no boundary"),
        ("Testify.swift", false, "lookalike"),
        ("TestBed.swift", false, "Test prefix only"),
        ("TestUtils.swift", false, "Test prefix util"),
        ("Manifest.swift", false, "manifest"),
        ("Digest.swift", false, "ends est"),
        ("Forest.swift", false, "ends est"),
        ("Request.swift", false, "ends est"),
        ("Attestation.swift", false, "at-prefix"),
        ("Greatest.swift", false, "lowercase test"),
        ("Contest.swift", false, "single word"),
        ("RequestTest.swift", true, "request test"),
        ("RequestTests.swift", true, "request tests"),
        ("XMLTests.swift", true, "acronym tests"),
        ("HTTPTest.swift", true, "http test"),
        ("APIServiceTests.swift", true, "api tests"),
        ("AssertTest.swift", true, "assert test"),
        ("Asserter.swift", false, "asserter"),
        ("FooTestData.swift", false, "TestData"),
        ("FooTestUtil.swift", false, "TestUtil"),
        ("FooTestable.swift", false, "Testable"),
        ("FooTesting.swift", false, "Testing"),
        ("Inspector.swift", false, "production"),
        ("Package.swift", false, "spm manifest — but ends in 'age' not test"),
        ("MyApp.swift", false, "main app"),
        ("ViewController.swift", false, "vc"),
        ("BaseTest.swift", true, "base test"),
        ("BaseSpec.swift", true, "base spec"),
        ("DataClassTests.swift", true, "data class tests"),
        ("HTTPTests.swift", true, "http tests"),
        ("APITest.swift", true, "api test"),
        ("foo_test.swift", true, "_test naming"),
        ("foo.test.swift", true, ".test naming"),
        // production + Test prefix
        ("TestRunner.swift", false, "Test prefix runner"),
        ("TestHelpers.swift", false, "Test prefix helper"),
        ("Spec.swift", true, "just Spec"),
        ("Specs.swift", true, "just Specs"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Objective-C — *Tests.m, *Test.m, Tests/ directory
// ==========================================================================
#[test]
fn objc_test_files() {
    let cases: Cases = &[
        ("FooTests.m", true, "objc tests"),
        ("FooTest.m", true, "objc test"),
        ("UserServiceTests.m", true, "service tests"),
        ("FooSpec.m", true, "spec"),
        ("FooSpecs.m", true, "specs"),
        ("Tests.m", true, "just Tests"),
        ("Test.m", true, "just Test"),
        ("Tests/Foo.m", true, "Tests dir"),
        ("project/Tests/Bar.m", true, "nested Tests"),
        // negatives
        ("Foo.m", false, "production"),
        ("ViewController.m", false, "vc"),
        ("AppDelegate.m", false, "delegate"),
        ("Latest.m", false, "lowercase test"),
        ("Contests.m", false, "single word"),
        ("Tester.m", false, "ends er"),
        ("Testing.m", false, "no boundary"),
        ("Testify.m", false, "lookalike"),
        ("TestBed.m", false, "Test prefix"),
        ("TestUtils.m", false, "Test prefix"),
        ("Manifest.m", false, "manifest"),
        ("Digest.m", false, "ends est"),
        ("Forest.m", false, "ends est"),
        ("Request.m", false, "ends est"),
        ("Attestation.m", false, "at-prefix"),
        ("Greatest.m", false, "lowercase test"),
        ("Contest.m", false, "single word"),
        ("RequestTest.m", true, "request test"),
        ("LatestTests.m", true, "latest tests"),
        ("XMLTest.m", true, "acronym test"),
        ("HTTPTest.m", true, "http test"),
        ("AssertTest.m", true, "assert test"),
        ("Asserter.m", false, "asserter"),
        ("FooTestData.m", false, "TestData"),
        ("FooTestUtil.m", false, "TestUtil"),
        ("FooTestable.m", false, "Testable"),
        ("FooTesting.m", false, "Testing"),
        ("Inspector.m", false, "production"),
        ("MyApp.m", false, "main"),
        ("BaseTest.m", true, "base test"),
        ("BaseSpec.m", true, "base spec"),
        ("DataClassTests.m", true, "data tests"),
        ("HTTPTests.m", true, "http tests"),
        ("APITest.m", true, "api test"),
        ("foo_test.m", true, "_test naming"),
        ("foo.test.m", true, ".test naming"),
        ("TestRunner.m", false, "Test prefix runner"),
        ("Spec.m", true, "just Spec"),
        ("project/src/Foo.m", false, "src dir"),
        ("project/Sources/Foo.m", false, "Sources dir"),
        ("./Tests/Bar.m", true, "relative Tests"),
        ("FooSpecial.m", false, "ends in cial"),
        ("Specifications.m", false, "spec substring no boundary"),
        // matrix
        ("BillingControllerTests.m", true, "billing tests"),
        ("BillingController.m", false, "billing controller"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Ruby — _spec.rb (RSpec), _test.rb (Minitest), spec/ test/ directories
// ==========================================================================
#[test]
fn ruby_test_files() {
    let cases: Cases = &[
        ("foo_spec.rb", true, "rspec spec"),
        ("user_spec.rb", true, "user rspec"),
        ("foo_test.rb", true, "minitest test"),
        ("api_test.rb", true, "minitest api"),
        ("spec/foo_spec.rb", true, "spec dir"),
        ("test/foo_test.rb", true, "test dir minitest"),
        ("tests/foo.rb", true, "tests dir"),
        ("spec/models/user_spec.rb", true, "rspec model"),
        ("spec/controllers/api_spec.rb", true, "rspec controller"),
        ("test/integration/api_test.rb", true, "minitest integration"),
        ("spec/spec_helper.rb", true, "rspec helper in spec dir"),
        ("/abs/path/spec/foo_spec.rb", true, "absolute spec path"),
        ("./spec/user_spec.rb", true, "relative spec"),
        // negatives
        ("foo.rb", false, "ordinary module"),
        ("user.rb", false, "model"),
        ("application.rb", false, "rails app"),
        ("application_controller.rb", false, "controller"),
        ("Gemfile", false, "no extension match"),
        ("Rakefile", false, "no extension match"),
        ("latest.rb", false, "lowercase test"),
        ("manifest.rb", false, "manifest"),
        ("contestant.rb", false, "test substring"),
        ("attestation.rb", false, "at-prefix"),
        ("forester.rb", false, "no boundary"),
        ("digester.rb", false, "no boundary"),
        ("requestor.rb", false, "no boundary"),
        ("test.rb", false, "single word"),
        ("testing.rb", false, "no separator"),
        ("testify.rb", false, "lookalike"),
        ("tester.rb", false, "ends er"),
        ("testbed.rb", false, "no separator"),
        ("greatest.rb", false, "lowercase"),
        ("contest.rb", false, "lowercase"),
        ("hottest.rb", false, "lowercase"),
        ("digest.rb", false, "ends est"),
        ("forest.rb", false, "ends est"),
        ("ingest.rb", false, "ends est"),
        ("conftest.rb", false, "Python file convention"),
        ("spec_helper.rb", false, "named like helper but at root"),
        ("rails_helper.rb", false, "rails helper"),
        // Rails patterns
        ("app/models/user.rb", false, "rails model"),
        ("app/controllers/api_controller.rb", false, "rails controller"),
        ("config/routes.rb", false, "rails config"),
        ("db/migrate/001_create.rb", false, "migration"),
        // factories often placed in spec/factories
        ("spec/factories/users.rb", true, "factories under spec"),
        ("spec/support/helpers.rb", true, "support under spec"),
        ("test/fixtures/users.yml", true, "anything in /test/ is test scope"),
        // negatives more
        ("spec.rb", false, "no separator"),
        ("specs.rb", false, "no separator"),
        ("hot_specs.rb", true, "_specs plural matches universal"),
        ("spec_runner.rb", false, "spec_runner production"),
        ("speculation.rb", false, "spec substring"),
        ("specification.rb", false, "spec substring"),
        ("foo_specs.rb", true, "_specs plural"),
        ("foo_tests.rb", true, "_tests plural"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// PHP — *Test.php, tests/ dir (PHPUnit)
// ==========================================================================
#[test]
fn php_test_files() {
    let cases: Cases = &[
        ("FooTest.php", true, "phpunit test"),
        ("FooTests.php", true, "tests plural"),
        ("UserServiceTest.php", true, "service test"),
        ("FooSpec.php", true, "spec"),
        ("FooSpecs.php", true, "specs"),
        ("Test.php", true, "just Test"),
        ("Tests.php", true, "just Tests"),
        ("tests/FooTest.php", true, "tests dir"),
        ("tests/Unit/UserTest.php", true, "tests/Unit"),
        ("tests/Feature/Api.php", true, "tests/Feature"),
        ("tests/Integration/Foo.php", true, "tests integration"),
        ("Tests/FooTest.php", true, "Tests dir capital"),
        ("project/tests/Foo.php", true, "nested tests"),
        ("/abs/path/tests/Foo.php", true, "absolute"),
        ("./tests/Foo.php", true, "relative"),
        // negatives
        ("Foo.php", false, "production"),
        ("Controller.php", false, "controller"),
        ("UserModel.php", false, "model"),
        ("Latest.php", false, "lowercase test"),
        ("Contests.php", false, "single word"),
        ("Tester.php", false, "ends er"),
        ("Testing.php", false, "no boundary"),
        ("Testify.php", false, "lookalike"),
        ("TestBed.php", false, "Test prefix only"),
        ("TestUtils.php", false, "Test prefix util"),
        ("TestCase.php", false, "TestCase ends in Case, not Test"),
        ("Manifest.php", false, "manifest"),
        ("Digest.php", false, "ends est"),
        ("Forest.php", false, "ends est"),
        ("Request.php", false, "ends est"),
        ("Attestation.php", false, "at-prefix"),
        ("Greatest.php", false, "lowercase test"),
        ("Contest.php", false, "single word"),
        ("RequestTest.php", true, "request test"),
        ("LatestTests.php", true, "latest tests"),
        ("XMLTest.php", true, "acronym test"),
        ("HTTPTest.php", true, "http test"),
        ("AssertTest.php", true, "assert test"),
        ("Asserter.php", false, "asserter"),
        ("FooTestData.php", false, "TestData"),
        ("FooTestable.php", false, "Testable"),
        ("FooTesting.php", false, "Testing"),
        ("Inspector.php", false, "production"),
        ("BaseTest.php", true, "base test"),
        ("BaseSpec.php", true, "base spec"),
        ("DataClassTests.php", true, "data tests"),
        ("HTTPTests.php", true, "http tests"),
        ("APITest.php", true, "api test"),
        ("foo_test.php", true, "_test naming"),
        ("foo.test.php", true, ".test naming"),
        ("foo_spec.php", true, "_spec naming"),
        ("Spec.php", true, "just Spec"),
        ("MyContest.php", false, "Contest"),
        ("ProtestSong.php", false, "Protest"),
        ("vendor/foo/Bar.php", false, "vendor production"),
        ("composer.json", false, "no php ext"),
        ("public/index.php", false, "entry"),
        ("config/app.php", false, "config"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// C — test_*.c, *_test.c, *_tests.c, tests/ directory
// ==========================================================================
#[test]
fn c_test_files() {
    let cases: Cases = &[
        ("test_foo.c", true, "test_ prefix"),
        ("test_parser.c", true, "test_ multi-word"),
        ("test_io.c", true, "test_ short"),
        ("foo_test.c", true, "_test suffix"),
        ("parser_test.c", true, "_test multi"),
        ("foo_tests.c", true, "_tests plural"),
        ("Test_Foo.c", true, "Test_ prefix capital"),
        ("test_main.h", true, "test header"),
        ("foo_test.h", true, "header _test"),
        // dir
        ("tests/foo.c", true, "tests dir"),
        ("test/foo.c", true, "test dir"),
        ("project/tests/parser.c", true, "nested tests"),
        ("./tests/io.c", true, "relative tests"),
        ("/abs/repo/tests/lib.c", true, "absolute"),
        // negatives
        ("foo.c", false, "production"),
        ("main.c", false, "main"),
        ("util.c", false, "util"),
        ("parser.c", false, "parser production"),
        ("foo.h", false, "header"),
        ("latest.c", false, "lowercase test"),
        ("manifest.c", false, "manifest"),
        ("contestant.c", false, "substring"),
        ("attestation.c", false, "at-prefix"),
        ("forester.c", false, "no boundary"),
        ("requestor.c", false, "no boundary"),
        ("digester.c", false, "no boundary"),
        ("test.c", false, "single word"),
        ("testing.c", false, "no separator"),
        ("testify.c", false, "lookalike"),
        ("tester.c", false, "ends er"),
        ("testbed.c", false, "no separator"),
        ("greatest.c", false, "lowercase"),
        ("contest.c", false, "lowercase"),
        ("hottest.c", false, "lowercase"),
        ("digest.c", false, "ends est"),
        ("forest.c", false, "ends est"),
        ("ingest.c", false, "ends est"),
        // edge cases
        ("test_data.c", true, "test_ prefix"),
        ("data_test.c", true, "_test suffix"),
        ("test_utils.c", true, "test_ utility — same convention"),
        ("test_helpers.h", true, "test_ helper header"),
        ("testdata.c", false, "no separator"),
        ("testdata_helper.c", false, "no test boundary"),
        // Linux kernel-style
        ("drivers/net/test_e1000.c", true, "kernel-style test"),
        ("drivers/net/e1000.c", false, "kernel driver"),
        ("kernel/sched/main.c", false, "kernel main"),
        ("lib/test_kasan.c", true, "lib test_"),
        // unusual
        ("test-foo.c", false, "hyphen not underscore"),
        ("foo-test.c", false, "hyphen not underscore"),
        ("Test.c", false, "just Test capital — Java convention not C"),
        ("Tests.c", false, "Tests capital — Java convention not C"),
        ("FooTest.c", false, "PascalCase Test — Java convention not C"),
        // builds/cmake
        ("CMakeLists.txt", false, "no c ext"),
        ("Makefile", false, "no c ext"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// C++ — same conventions as C plus PascalCase via Java-ish suffixes
// ==========================================================================
#[test]
fn cpp_test_files() {
    let cases: Cases = &[
        ("test_foo.cpp", true, "test_ prefix"),
        ("test_parser.cpp", true, "test_ multi"),
        ("foo_test.cpp", true, "_test suffix"),
        ("foo_tests.cpp", true, "_tests plural"),
        ("Test_Foo.cpp", true, "Test_ capital"),
        ("foo_test.cc", true, "_test cc"),
        ("foo_test.cxx", true, "_test cxx"),
        ("foo_test.hpp", true, "_test hpp"),
        ("foo_test.hxx", true, "_test hxx"),
        ("foo_test.hh", true, "_test hh"),
        // dir
        ("tests/foo.cpp", true, "tests dir cpp"),
        ("test/foo.cpp", true, "test dir cpp"),
        ("project/tests/main.cpp", true, "nested tests cpp"),
        // gtest naming
        ("foo_unittest.cpp", false, "unittest suffix not in rules"),
        ("foo_test.cpp", true, "underscore test"),
        // negatives
        ("foo.cpp", false, "production"),
        ("main.cpp", false, "main"),
        ("util.cpp", false, "util"),
        ("parser.cpp", false, "parser"),
        ("foo.hpp", false, "header"),
        ("latest.cpp", false, "lowercase test"),
        ("manifest.cpp", false, "manifest"),
        ("contestant.cpp", false, "substring"),
        ("attestation.cpp", false, "at-prefix"),
        ("forester.cpp", false, "no boundary"),
        ("requestor.cpp", false, "no boundary"),
        ("digester.cpp", false, "no boundary"),
        ("test.cpp", false, "single word"),
        ("testing.cpp", false, "no separator"),
        ("testify.cpp", false, "lookalike"),
        ("tester.cpp", false, "ends er"),
        ("testbed.cpp", false, "no separator"),
        ("greatest.cpp", false, "lowercase"),
        ("contest.cpp", false, "lowercase"),
        ("hottest.cpp", false, "lowercase"),
        ("digest.cpp", false, "ends est"),
        ("forest.cpp", false, "ends est"),
        ("ingest.cpp", false, "ends est"),
        // edge cases
        ("test_data.cpp", true, "test_ data"),
        ("data_test.cpp", true, "data _test"),
        ("test_utils.cpp", true, "test_ utility"),
        ("testdata.cpp", false, "no separator"),
        ("test-foo.cpp", false, "hyphen not underscore"),
        ("foo-test.cpp", false, "hyphen not underscore"),
        // PascalCase suffix not C++ convention — should NOT match
        ("FooTest.cpp", false, "PascalCase Test — Java convention not C++"),
        ("FooTests.cpp", false, "Tests capital not C++ convention"),
        ("FooSpec.cpp", false, "Spec capital not C++ convention"),
        // dirs again
        ("./tests/io.cpp", true, "relative tests"),
        ("/abs/repo/tests/lib.cpp", true, "absolute tests"),
        ("vendor/foo/bar.cpp", false, "vendor production"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Zig — test_*.zig and tests/ dir (inline test "" {} not detectable)
// ==========================================================================
#[test]
fn zig_test_files() {
    let cases: Cases = &[
        ("test_foo.zig", true, "test_ prefix"),
        ("test_parser.zig", true, "test_ multi"),
        ("test_io.zig", true, "test_ short"),
        ("foo_test.zig", true, "_test suffix"),
        ("parser_test.zig", true, "_test multi"),
        ("foo_tests.zig", true, "_tests plural"),
        ("tests/foo.zig", true, "tests dir"),
        ("test/foo.zig", true, "test dir"),
        ("project/tests/parser.zig", true, "nested tests"),
        // negatives
        ("foo.zig", false, "production"),
        ("main.zig", false, "main"),
        ("build.zig", false, "build script"),
        ("std.zig", false, "stdlib"),
        ("latest.zig", false, "lowercase"),
        ("manifest.zig", false, "manifest"),
        ("contestant.zig", false, "substring"),
        ("attestation.zig", false, "at-prefix"),
        ("test.zig", false, "single word"),
        ("testing.zig", false, "no separator"),
        ("tester.zig", false, "ends er"),
        ("testbed.zig", false, "no separator"),
        ("testify.zig", false, "lookalike"),
        ("greatest.zig", false, "lowercase"),
        ("contest.zig", false, "lowercase"),
        ("hottest.zig", false, "lowercase"),
        ("digest.zig", false, "ends est"),
        ("forest.zig", false, "ends est"),
        // inline tests in production code — undetectable at file level
        ("src/parser.zig", false, "inline test {} not file-level"),
        ("src/lib.zig", false, "lib with inline tests"),
        // edge cases
        ("test_data.zig", true, "test_ data"),
        ("data_test.zig", true, "_test data"),
        ("testdata.zig", false, "no separator"),
        ("test-foo.zig", false, "hyphen not underscore"),
        ("foo-test.zig", false, "hyphen not underscore"),
        ("FooTest.zig", false, "PascalCase Java convention"),
        ("Test.zig", false, "single PascalCase"),
        // dirs
        ("./tests/io.zig", true, "relative tests"),
        ("/abs/repo/tests/lib.zig", true, "absolute tests"),
        ("examples/demo.zig", false, "examples dir not test"),
        ("benches/bench.zig", false, "bench dir not test"),
        // build.zig.zon variations
        ("build.zig.zon", false, "no .zig extension"),
        // production lookalikes
        ("requestor.zig", false, "no boundary"),
        ("forester.zig", false, "no boundary"),
        ("digester.zig", false, "no boundary"),
        ("ingestor.zig", false, "no boundary"),
        ("specifications.zig", false, "spec substring"),
        ("speculation.zig", false, "spec substring"),
        ("test_helpers.zig", true, "test_ helper"),
        ("test_utils.zig", true, "test_ utility"),
        ("setup.zig", false, "setup"),
        ("config.zig", false, "config"),
        ("util.zig", false, "util"),
        ("conf_test.zig", true, "_test naming"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Tcl — .test files (tcltest), tests/ dir
// ==========================================================================
#[test]
fn tcl_test_files() {
    let cases: Cases = &[
        ("foo.test", false, ".test ext not in language map"),
        ("tests/foo.tcl", true, "tests dir"),
        ("test/foo.tcl", true, "test dir"),
        ("foo_test.tcl", true, "_test naming"),
        ("foo.test.tcl", true, ".test. naming"),
        ("test_foo.tcl", false, "test_ prefix not Tcl convention"),
        ("foo_spec.tcl", true, "_spec naming"),
        ("project/tests/api.tcl", true, "nested tests"),
        ("./tests/io.tcl", true, "relative"),
        ("/abs/path/tests/foo.tcl", true, "absolute"),
        // negatives
        ("foo.tcl", false, "production"),
        ("main.tcl", false, "main"),
        ("init.tcl", false, "init"),
        ("foo.tk", false, "tk script"),
        ("latest.tcl", false, "lowercase test"),
        ("manifest.tcl", false, "manifest"),
        ("contestant.tcl", false, "substring"),
        ("attestation.tcl", false, "at-prefix"),
        ("test.tcl", false, "single word"),
        ("testing.tcl", false, "no separator"),
        ("tester.tcl", false, "ends er"),
        ("testbed.tcl", false, "no separator"),
        ("testify.tcl", false, "lookalike"),
        ("greatest.tcl", false, "lowercase"),
        ("contest.tcl", false, "lowercase"),
        ("hottest.tcl", false, "lowercase"),
        ("digest.tcl", false, "ends est"),
        ("forest.tcl", false, "ends est"),
        ("ingest.tcl", false, "ends est"),
        ("FooTest.tcl", false, "PascalCase Java convention"),
        ("Test.tcl", false, "single PascalCase"),
        ("requestor.tcl", false, "no boundary"),
        ("forester.tcl", false, "no boundary"),
        ("digester.tcl", false, "no boundary"),
        ("specifications.tcl", false, "spec substring"),
        ("speculation.tcl", false, "spec substring"),
        // itcl extension
        ("foo.itcl", false, "itcl production"),
        ("foo_test.itcl", true, "itcl _test"),
        ("tests/widget.itcl", true, "tests itcl"),
        // pkgIndex / loaders
        ("pkgIndex.tcl", false, "package loader"),
        ("loader.tcl", false, "loader"),
        // Tk widgets
        ("widgets.tk", false, "widgets"),
        ("button.tk", false, "button"),
        // production patterns
        ("./src/main.tcl", false, "src main"),
        ("project/src/api.tcl", false, "project src"),
        // edge: test inside non-test dir
        ("config/test_helpers.tcl", false, "test_helpers production helper"),
        ("config/foo.tcl", false, "config"),
        // dir-based wins
        ("config/tests/foo.tcl", true, "config/tests still test dir"),
        ("vendor/lib/foo.tcl", false, "vendor production"),
        ("test_runner.tcl", false, "test_runner production"),
        ("foo.tests.tcl", true, ".tests. naming"),
        ("foo.specs.tcl", true, ".specs. naming"),
        ("test_data.tcl", false, "test_ prefix not Tcl convention"),
        ("examples/demo.tcl", false, "examples"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Haskell — *Spec.hs (HSpec), *Test.hs, test/ tests/ dirs
// ==========================================================================
#[test]
fn haskell_test_files() {
    let cases: Cases = &[
        ("FooSpec.hs", true, "hspec spec"),
        ("FooSpecs.hs", true, "specs plural"),
        ("FooTest.hs", true, "test"),
        ("FooTests.hs", true, "tests plural"),
        ("Spec.hs", true, "just Spec"),
        ("Specs.hs", true, "just Specs"),
        ("Test.hs", true, "just Test"),
        ("Tests.hs", true, "just Tests"),
        ("test/Foo.hs", true, "test dir"),
        ("tests/Foo.hs", true, "tests dir"),
        ("project/test/Bar.hs", true, "nested test"),
        ("project/tests/Spec.hs", true, "nested tests"),
        ("./test/Foo.hs", true, "relative test"),
        ("/abs/path/tests/Foo.hs", true, "absolute"),
        // negatives
        ("Foo.hs", false, "production"),
        ("Main.hs", false, "main"),
        ("Lib.hs", false, "library"),
        ("Setup.hs", false, "cabal setup"),
        ("Latest.hs", false, "lowercase test"),
        ("Contests.hs", false, "single word"),
        ("Tester.hs", false, "ends er"),
        ("Testing.hs", false, "no boundary"),
        ("Testify.hs", false, "lookalike"),
        ("TestBed.hs", false, "Test prefix only"),
        ("TestUtils.hs", false, "Test prefix util"),
        ("Manifest.hs", false, "manifest"),
        ("Digest.hs", false, "ends est"),
        ("Forest.hs", false, "ends est"),
        ("Request.hs", false, "ends est"),
        ("Attestation.hs", false, "at-prefix"),
        ("Greatest.hs", false, "lowercase"),
        ("Contest.hs", false, "single word"),
        ("RequestTest.hs", true, "request test"),
        ("LatestTests.hs", true, "latest tests"),
        ("XMLTest.hs", true, "acronym"),
        ("HTTPTest.hs", true, "http test"),
        ("DataClassSpec.hs", true, "data class spec"),
        ("FooTestData.hs", false, "TestData"),
        ("FooTestable.hs", false, "Testable"),
        ("FooTesting.hs", false, "Testing"),
        // .lhs literate
        ("Foo.lhs", false, "literate production"),
        ("FooSpec.lhs", true, "literate spec"),
        // common patterns
        ("Data/List.hs", false, "Data.List module"),
        ("Network/HTTP.hs", false, "Network module"),
        ("test/SpecHelper.hs", true, "spec helper in test dir"),
        ("Data.hs", false, "Data module"),
        ("MyContest.hs", false, "Contest"),
        ("ProtestSong.hs", false, "Protest"),
        // negatives with substring
        ("Forester.hs", false, "no boundary"),
        ("Digester.hs", false, "no boundary"),
        ("Requestor.hs", false, "no boundary"),
        // package config
        ("Cabal.hs", false, "cabal lib"),
        ("Setup.hs", false, "setup script"),
        ("foo.cabal", false, "no hs ext"),
        ("stack.yaml", false, "no hs ext"),
        // assertions
        ("AssertSpec.hs", true, "assert spec"),
        ("AssertTest.hs", true, "assert test"),
        ("Asserter.hs", false, "asserter"),
        ("ListSpec.hs", true, "list spec"),
        ("ListTests.hs", true, "list tests"),
        ("ListTesting.hs", false, "list testing"),
        ("BaseTest.hs", true, "base test"),
        ("BaseSpec.hs", true, "base spec"),
        // path-only triggers
        ("./tests/sub/Bar.hs", true, "deep tests"),
        ("./test/sub/Bar.hs", true, "deep test"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// Lua — *_spec.lua (busted), *_test.lua, test_*.lua, spec_*.lua
// ==========================================================================
#[test]
fn lua_test_files() {
    let cases: Cases = &[
        ("foo_spec.lua", true, "busted spec"),
        ("user_spec.lua", true, "user spec"),
        ("foo_test.lua", true, "test suffix"),
        ("test_foo.lua", true, "test_ prefix"),
        ("spec_foo.lua", true, "spec_ prefix"),
        ("spec/foo_spec.lua", true, "spec dir"),
        ("test/foo_test.lua", true, "test dir"),
        ("tests/foo.lua", true, "tests dir"),
        ("./spec/api_spec.lua", true, "relative spec"),
        ("/abs/spec/foo_spec.lua", true, "absolute spec"),
        // negatives
        ("foo.lua", false, "production"),
        ("main.lua", false, "main"),
        ("init.lua", false, "init"),
        ("config.lua", false, "config"),
        ("latest.lua", false, "lowercase test"),
        ("manifest.lua", false, "manifest"),
        ("contestant.lua", false, "substring"),
        ("attestation.lua", false, "at-prefix"),
        ("test.lua", false, "single word"),
        ("testing.lua", false, "no separator"),
        ("tester.lua", false, "ends er"),
        ("testbed.lua", false, "no separator"),
        ("testify.lua", false, "lookalike"),
        ("greatest.lua", false, "lowercase"),
        ("contest.lua", false, "lowercase"),
        ("hottest.lua", false, "lowercase"),
        ("digest.lua", false, "ends est"),
        ("forest.lua", false, "ends est"),
        ("ingest.lua", false, "ends est"),
        // edge cases
        ("test-foo.lua", false, "hyphen not underscore"),
        ("FooTest.lua", false, "PascalCase Java convention"),
        ("Test.lua", false, "single PascalCase"),
        ("requestor.lua", false, "no boundary"),
        ("forester.lua", false, "no boundary"),
        ("digester.lua", false, "no boundary"),
        ("specifications.lua", false, "spec substring"),
        ("speculation.lua", false, "spec substring"),
        // neovim plugin patterns
        ("plugin/foo.lua", false, "neovim plugin"),
        ("lua/myplugin/init.lua", false, "neovim init"),
        ("after/plugin/foo.lua", false, "neovim after"),
        // tests in plugin
        ("tests/myplugin_spec.lua", true, "tests with spec"),
        ("spec/myplugin_spec.lua", true, "spec dir"),
        // openresty/lua_modules
        ("lua_modules/foo.lua", false, "vendored"),
        // production helpers
        ("test_runner.lua", true, "test_ prefix — caught by Lua rule"),
        ("test_helpers.lua", true, "test_ prefix"),
        ("spec_runner.lua", true, "spec_ prefix"),
        ("spec_helpers.lua", true, "spec_ prefix"),
        // negatives with test substr
        ("foo_speculative.lua", false, "speculative"),
        ("foo_specification.lua", false, "specification"),
        ("foo_specs.lua", true, "_specs plural"),
        ("foo_tests.lua", true, "_tests plural"),
        ("foo.spec.lua", true, ".spec. naming"),
        ("foo.test.lua", true, ".test. naming"),
        // weird
        ("luatest.lua", false, "no separator"),
        ("luaunit.lua", false, "framework not test"),
        ("busted.lua", false, "framework"),
        ("LICENSE", false, "no extension"),
        ("rockspec.lua", false, "lua rocks"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// R — testthat: tests/testthat/test-*.R, test_*.R
// ==========================================================================
#[test]
fn r_test_files() {
    let cases: Cases = &[
        ("test-foo.R", true, "testthat test-"),
        ("test-user-auth.R", true, "testthat multi-word"),
        ("test_foo.R", true, "test_ prefix"),
        ("test_parser.R", true, "test_ multi"),
        ("tests/testthat/test-api.R", true, "testthat dir"),
        ("tests/test_foo.R", true, "tests dir"),
        ("test/foo.R", true, "test dir"),
        ("./tests/testthat/test-foo.R", true, "relative testthat"),
        ("/abs/repo/tests/testthat/test-foo.R", true, "absolute testthat"),
        ("test-foo.r", true, "lowercase ext"),
        ("test_foo.r", true, "lowercase ext _"),
        // negatives
        ("foo.R", false, "production"),
        ("main.R", false, "main"),
        ("script.R", false, "script"),
        ("analysis.R", false, "analysis"),
        ("latest.R", false, "lowercase test"),
        ("manifest.R", false, "manifest"),
        ("contestant.R", false, "substring"),
        ("attestation.R", false, "at-prefix"),
        ("test.R", false, "single word"),
        ("testing.R", false, "no separator"),
        ("tester.R", false, "ends er"),
        ("testbed.R", false, "no separator"),
        ("testify.R", false, "lookalike"),
        ("greatest.R", false, "lowercase"),
        ("contest.R", false, "lowercase"),
        ("hottest.R", false, "lowercase"),
        ("digest.R", false, "ends est"),
        ("forest.R", false, "ends est"),
        ("ingest.R", false, "ends est"),
        // R package conventions
        ("DESCRIPTION", false, "no R ext"),
        ("NAMESPACE", false, "no R ext"),
        ("R/foo.R", false, "R/ source dir not tests"),
        ("R/utils.R", false, "R/ utils"),
        // shiny apps
        ("app.R", false, "shiny app"),
        ("server.R", false, "shiny server"),
        ("ui.R", false, "shiny ui"),
        // tests dir wins
        ("tests/test-other-naming.R", true, "tests dir override"),
        ("tests/foo.R", true, "tests dir even without test- prefix"),
        // negatives
        ("test-data.R", true, "test- prefix"),
        ("foo-test.R", false, "no test- prefix nor _test."),
        ("foo_test.R", true, "_test suffix universal"),
        ("data-test.R", false, "no convention"),
        ("Test.R", false, "PascalCase not R convention"),
        ("FooTest.R", false, "PascalCase Java convention"),
        // production lookalikes
        ("requestor.R", false, "no boundary"),
        ("forester.R", false, "no boundary"),
        ("digester.R", false, "no boundary"),
        ("specifications.R", false, "spec substring"),
        ("speculation.R", false, "spec substring"),
        // helpers
        ("tests/testthat/helper-foo.R", true, "testthat helper in tests"),
        ("tests/testthat/setup.R", true, "testthat setup in tests"),
        ("test_helpers.R", true, "test_ helper"),
        ("test-helpers.R", true, "test- helper"),
        ("helpers.R", false, "production helpers"),
        ("setup.R", false, "production setup"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// COBOL — limited test convention; primarily directory-based
// ==========================================================================
#[test]
fn cobol_test_files() {
    let cases: Cases = &[
        // dir-based
        ("tests/PROG.cob", true, "tests dir"),
        ("test/PROG.cob", true, "test dir"),
        ("project/tests/MAIN.cbl", true, "nested tests cbl"),
        ("project/tests/UTIL.cobol", true, "nested tests cobol"),
        ("./tests/PROG.cob", true, "relative tests"),
        ("/abs/path/tests/PROG.cob", true, "absolute tests"),
        // suffix universal
        ("PROG_test.cob", true, "_test suffix"),
        ("MAIN_TEST.cob", false, "uppercase TEST not _test."),
        ("prog_test.cob", true, "lowercase _test"),
        ("foo.test.cob", true, ".test. naming"),
        ("foo_spec.cob", true, "_spec naming"),
        // negatives
        ("PROG.cob", false, "production cobol"),
        ("MAIN.cbl", false, "main cobol"),
        ("UTIL.cobol", false, "util cobol"),
        ("foo.cob", false, "production"),
        ("foo.cbl", false, "production"),
        ("LATEST.cob", false, "lowercase test substring (case-sensitive miss)"),
        ("CONTEST.cob", false, "no boundary"),
        ("HOTTEST.cob", false, "no boundary"),
        ("MANIFEST.cob", false, "manifest"),
        ("DIGEST.cob", false, "ends est"),
        ("FOREST.cob", false, "ends est"),
        ("INGEST.cob", false, "ends est"),
        ("REQUEST.cob", false, "ends est"),
        ("ATTEST.cob", false, "at-prefix"),
        ("PROTEST.cob", false, "no test boundary"),
        ("TESTER.cob", false, "ends er"),
        ("TESTING.cob", false, "no separator"),
        ("TESTBED.cob", false, "no separator"),
        ("TEST.cob", false, "single word"),
        ("PROG.cobol", false, "production cobol full ext"),
        // dirs with mixed case — only specific ones supported
        ("project/test/util.cob", true, "test dir"),
        ("project/Tests/PROG.cob", true, "Tests capital"),
        ("./tests/main.cbl", true, "relative tests cbl"),
        ("project/spec/PROG.cob", true, "spec dir"),
        // production paths
        ("src/PROG.cob", false, "src dir cobol"),
        ("./src/main.cbl", false, "src cbl"),
        // weird
        ("FOO_test.cbl", true, "uppercase prefix _test lowercase suffix"),
        ("FOO_TEST.cbl", false, "all uppercase _TEST not _test."),
        ("FOO.test.cbl", true, ".test. naming"),
        ("FOO_spec.cbl", true, "_spec suffix"),
        ("foo.spec.cobol", true, ".spec. cobol"),
        ("vendor/PROG.cob", false, "vendor production"),
        ("./tests/sub/foo.cob", true, "deep tests"),
        ("./test/sub/foo.cob", true, "deep test"),
        ("foo_tests.cob", true, "_tests plural"),
        ("foo_specs.cob", true, "_specs plural"),
        ("foo.tests.cob", true, ".tests. plural"),
        ("foo.specs.cob", true, ".specs. plural"),
        ("MANIFEST.cobol", false, "manifest"),
        ("MAIN.cbl", false, "main"),
        ("CONFIG.cobol", false, "config"),
        ("TESTPROG.cob", false, "no separator"),
        ("PROG_T.cob", false, "abbreviated"),
        ("DATA.cob", false, "data prog"),
        ("REPORT.cob", false, "report prog"),
    ];
    assert_cases(cases);
}

// ==========================================================================
// D — tests/ dir + test_*.d (inline unittest blocks not detectable)
// ==========================================================================
#[test]
fn d_test_files() {
    let cases: Cases = &[
        ("test_foo.d", true, "test_ prefix"),
        ("test_parser.d", true, "test_ multi"),
        ("foo_test.d", true, "_test suffix"),
        ("foo_tests.d", true, "_tests plural"),
        ("tests/foo.d", true, "tests dir"),
        ("test/foo.d", true, "test dir"),
        ("project/tests/main.d", true, "nested tests"),
        ("./tests/io.d", true, "relative tests"),
        ("/abs/repo/tests/lib.d", true, "absolute tests"),
        // negatives
        ("foo.d", false, "production"),
        ("main.d", false, "main"),
        ("util.d", false, "util"),
        ("foo.di", false, "interface file"),
        ("latest.d", false, "lowercase"),
        ("manifest.d", false, "manifest"),
        ("contestant.d", false, "substring"),
        ("attestation.d", false, "at-prefix"),
        ("test.d", false, "single word"),
        ("testing.d", false, "no separator"),
        ("tester.d", false, "ends er"),
        ("testbed.d", false, "no separator"),
        ("testify.d", false, "lookalike"),
        ("greatest.d", false, "lowercase"),
        ("contest.d", false, "lowercase"),
        ("hottest.d", false, "lowercase"),
        ("digest.d", false, "ends est"),
        ("forest.d", false, "ends est"),
        ("ingest.d", false, "ends est"),
        // inline unittest in production
        ("src/parser.d", false, "inline unittest not file-level"),
        ("src/lib.d", false, "lib with inline unittest"),
        // edge cases
        ("test_data.d", true, "test_ data"),
        ("data_test.d", true, "_test data"),
        ("testdata.d", false, "no separator"),
        ("test-foo.d", false, "hyphen not underscore"),
        ("foo-test.d", false, "hyphen not underscore"),
        ("FooTest.d", false, "PascalCase Java convention"),
        ("Test.d", false, "single PascalCase"),
        ("requestor.d", false, "no boundary"),
        ("forester.d", false, "no boundary"),
        ("digester.d", false, "no boundary"),
        // dub config
        ("dub.json", false, "no d ext"),
        ("dub.sdl", false, "no d ext"),
        // examples / source
        ("examples/demo.d", false, "examples"),
        ("source/myproject/main.d", false, "dub source layout"),
        ("source/myproject/test_runner.d", true, "test_ in source"),
        ("source/myproject/tests/foo.d", true, "tests in source"),
        ("test_helpers.d", true, "test_ helper"),
        ("test_utils.d", true, "test_ util"),
        ("conf_test.d", true, "_test suffix"),
        ("specifications.d", false, "spec substring"),
        ("speculation.d", false, "spec substring"),
        // .di files
        ("foo_test.di", true, "_test di interface"),
        ("foo.di", false, "di production"),
        ("test_foo.di", true, "test_ prefix .di applies"),
    ];
    assert_cases(cases);
}
