// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "MyApp",
    dependencies: [
        .package(url: "https://github.com/Alamofire/Alamofire.git", from: "5.8.0"),
        .package(url: "https://github.com/apple/swift-log.git", .upToNextMajor(from: "1.5.0")),
        .package(name: "Renamed", url: "https://github.com/foo/bar.git", from: "2.0.0"),
        .package(path: "../LocalLib"),
    ],
    targets: [
        .target(name: "MyApp", dependencies: [
            .product(name: "Alamofire", package: "Alamofire"),
        ]),
    ]
)
