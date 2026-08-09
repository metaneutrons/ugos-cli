# Changelog

## [0.3.4](https://github.com/metaneutrons/ugos-cli/compare/v0.3.3...v0.3.4) (2026-08-09)


### Bug Fixes

* **docker:** accept null container list fields ([ecc0bd0](https://github.com/metaneutrons/ugos-cli/commit/ecc0bd09d123df929debab6bb0aa570cc6d000a7))

## [0.3.3](https://github.com/metaneutrons/ugos-cli/compare/v0.3.2...v0.3.3) (2026-08-07)


### Bug Fixes

* ComposeProject.containerList can be null right after creation ([da6b772](https://github.com/metaneutrons/ugos-cli/commit/da6b772bd39f41aceca8cc6e927ab8ab9b7d9b42))

## [0.3.2](https://github.com/metaneutrons/ugos-cli/compare/v0.3.1...v0.3.2) (2026-08-07)


### Bug Fixes

* don't report false failure when CreateContainer needs to pull first ([1f5eb6e](https://github.com/metaneutrons/ugos-cli/commit/1f5eb6e1bdfc739490464fcdd3b8184d7eae5d7a))
* GetDockerSharedFolder response is {result: string}, not a bare string ([3f99ddf](https://github.com/metaneutrons/ugos-cli/commit/3f99ddf546cdfbf6f6f72db4cddddedd822eb009))

## [0.3.1](https://github.com/metaneutrons/ugos-cli/compare/v0.3.0...v0.3.1) (2026-08-07)


### Bug Fixes

* send full image:tag string in CreateContainer, not just repo name ([a68fb4a](https://github.com/metaneutrons/ugos-cli/commit/a68fb4a996b569f84e3fc8558ee82e3be8f1b63e))

## [0.3.0](https://github.com/metaneutrons/ugos-cli/compare/v0.2.0...v0.3.0) (2026-08-07)


### Features

* add Docker Compose project management (create/list/show/start/stop/restart/remove) ([126b5a0](https://github.com/metaneutrons/ugos-cli/commit/126b5a03ed5cc71dab957a2abfd7e597f9ecaa72))
* add Docker container and image management ([9cfc4f4](https://github.com/metaneutrons/ugos-cli/commit/9cfc4f4542dbbb53400fc538ab0b9e9659a0f54b))
* add Docker container create, show ([8627b7f](https://github.com/metaneutrons/ugos-cli/commit/8627b7f28fa353ae271864ae860e5c34108a3988))
* add VM create/update and OVA export/import ([7d24c51](https://github.com/metaneutrons/ugos-cli/commit/7d24c51b580441470a36116a224c43a88f0e7faa))
* complete KVM and Docker to 100% coverage ([c6e13e3](https://github.com/metaneutrons/ugos-cli/commit/c6e13e3ecd2dfc07e056f5331f17636ef6f29dcf))
* complete KVM API coverage ([ffe4d7a](https://github.com/metaneutrons/ugos-cli/commit/ffe4d7afd53e57843a21690cded21b5bdb1f11d6))
* early validation for create flags ([2e096b7](https://github.com/metaneutrons/ugos-cli/commit/2e096b7d49f04c957aa0de5b5c4015679a0b2697))
* replace JSON file create/update with proper CLI flags ([8497174](https://github.com/metaneutrons/ugos-cli/commit/849717440f5e715cefbc60bd34cfe0f9059279e2))


### Bug Fixes

* correct CreateContainer port mapping fields and add missing gpuIds/subnetSettings ([e7e8026](https://github.com/metaneutrons/ugos-cli/commit/e7e8026ce557cd47e83919d32a1f0b0617cb6140))
* correct Docker container/image types from real API responses ([85b150a](https://github.com/metaneutrons/ugos-cli/commit/85b150a7706c92acf0b4fa82dd866338532e51a2))
* correct Docker image type fields and image download body ([250f42a](https://github.com/metaneutrons/ugos-cli/commit/250f42a003c4804a2510df6fdf17ef0b85f4f6c0))
* enterprise-grade hardening ([6a418dc](https://github.com/metaneutrons/ugos-cli/commit/6a418dc0dcec8395b68ac6a77e293db6d2b7b645))
* hide password value in CLI help output ([19d0e6f](https://github.com/metaneutrons/ugos-cli/commit/19d0e6fc376222d23439d90f4454caad8007414f))
* MCP tool schemas — use plain string type instead of nullable ([65b5a86](https://github.com/metaneutrons/ugos-cli/commit/65b5a86a526c5d6d9c899f6b54e6110907ba720d))
* split too-long doc comment paragraph on SubnetSetting ([0d32767](https://github.com/metaneutrons/ugos-cli/commit/0d3276790e27a9ebe18e015f1060af350be21230))
* unbreak CI clippy --all-targets (unwrap/panic/module_inception in tests) ([52c5c09](https://github.com/metaneutrons/ugos-cli/commit/52c5c096b23c8f91c4072b012ae7160681d86bc1))
* wire tool_handler to ServerHandler — MCP tools now actually work ([d241044](https://github.com/metaneutrons/ugos-cli/commit/d241044600938ccb4f153695523eafa1dcbb27ad))

## [0.2.0](https://github.com/metaneutrons/ugos-cli/compare/v0.1.0...v0.2.0) (2026-04-12)


### Features

* add Homebrew tap publishing to release workflow ([8df6239](https://github.com/metaneutrons/ugos-cli/commit/8df623953f78c710bcd2d730f75060e6a96a040f))
* initial implementation ([1fcdcc3](https://github.com/metaneutrons/ugos-cli/commit/1fcdcc36b5e0f38f398baa61c7098537222a4778))


### Bug Fixes

* clean up Homebrew formula generation in release workflow ([80af366](https://github.com/metaneutrons/ugos-cli/commit/80af3663d4c90db91f8f243de198e6f72d85fa36))
