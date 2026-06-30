use std::collections::HashSet;
use std::path::PathBuf;

use pulse_audit::imports::resolve_by_suffix;
use pulse_syntax::parse::Language;

fn typed_set<I: IntoIterator<Item = &'static str>>(paths: I) -> HashSet<PathBuf> {
    paths.into_iter().map(PathBuf::from).collect()
}

#[test]
fn java_dotted_resolves_via_suffix() {
    let typed =
        typed_set(["/repo/src/main/java/com/example/foo/Bar.java", "/repo/src/main/java/com/example/baz/Qux.java"]);
    let resolved = resolve_by_suffix("com.example.foo.Bar", Language::Java, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/src/main/java/com/example/foo/Bar.java")));
}

#[test]
fn kotlin_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/src/main/kotlin/com/example/Service.kt"]);
    let resolved = resolve_by_suffix("com.example.Service", Language::Kotlin, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/src/main/kotlin/com/example/Service.kt")));
}

#[test]
fn csharp_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/src/MyApp/Services/Foo.cs"]);
    let resolved = resolve_by_suffix("MyApp.Services.Foo", Language::CSharp, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/src/MyApp/Services/Foo.cs")));
}

#[test]
fn swift_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/Sources/App/Network/Client.swift"]);
    let resolved = resolve_by_suffix("App.Network.Client", Language::Swift, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/Sources/App/Network/Client.swift")));
}

#[test]
fn haskell_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/src/Data/Foo/Bar.hs"]);
    let resolved = resolve_by_suffix("Data.Foo.Bar", Language::Haskell, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/src/Data/Foo/Bar.hs")));
}

#[test]
fn d_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/source/app/util/parse.d"]);
    let resolved = resolve_by_suffix("app.util.parse", Language::D, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/source/app/util/parse.d")));
}

#[test]
fn groovy_dotted_resolves_via_suffix() {
    let typed = typed_set(["/repo/src/main/groovy/com/co/Service.groovy"]);
    let resolved = resolve_by_suffix("com.co.Service", Language::Groovy, &typed);
    assert_eq!(resolved, Some(PathBuf::from("/repo/src/main/groovy/com/co/Service.groovy")));
}

#[test]
fn unsupported_language_returns_none() {
    let typed = typed_set(["/repo/foo.rs"]);
    assert!(resolve_by_suffix("foo", Language::Rust, &typed).is_none());
    assert!(resolve_by_suffix("foo", Language::Go, &typed).is_none());
    assert!(resolve_by_suffix("foo", Language::JavaScript, &typed).is_none());
}

#[test]
fn empty_typed_set_returns_none() {
    let typed = HashSet::new();
    assert!(resolve_by_suffix("com.example.foo.Bar", Language::Java, &typed).is_none());
}

#[test]
fn no_match_returns_none() {
    let typed = typed_set(["/repo/src/main/java/com/example/Other.java"]);
    assert!(resolve_by_suffix("com.example.foo.Bar", Language::Java, &typed).is_none());
}

#[test]
fn extension_matters_for_suffix() {
    let typed = typed_set(["/repo/src/com/example/Bar.kt"]);
    assert!(
        resolve_by_suffix("com.example.Bar", Language::Java, &typed).is_none(),
        "Java suffix should not match a .kt file"
    );
    assert_eq!(
        resolve_by_suffix("com.example.Bar", Language::Kotlin, &typed),
        Some(PathBuf::from("/repo/src/com/example/Bar.kt"))
    );
}
