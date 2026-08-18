# Changelog

## [0.8.0](https://github.com/metaneutrons/ugos-cli/compare/v0.7.0...v0.8.0) (2026-08-18)


### ⚠ BREAKING CHANGES

* **deps:** the minimum supported Rust version is now 1.88.

### Bug Fixes

* **deps:** raise MSRV to 1.88 so time can be updated ([1e069d6](https://github.com/metaneutrons/ugos-cli/commit/1e069d60297a2fe9a3bcc775f907277dfb3fae53))

## [0.7.0](https://github.com/metaneutrons/ugos-cli/compare/v0.6.0...v0.7.0) (2026-08-18)


### ⚠ BREAKING CHANGES

* **log:** `ugos log` is now the system log. The KVM audit log it used to show is `ugos vm log`.

### Features

* **client:** encrypted requests now work ([dab3d9b](https://github.com/metaneutrons/ugos-cli/commit/dab3d9b422cda6a0c27804cc0621aceb19b5360e))
* **client:** implement UGOS request encryption (not yet accepted) ([a1c6568](https://github.com/metaneutrons/ugos-cli/commit/a1c6568c33a5aa62cd03a9f7571b0d1fb21becdc))
* **download:** queue downloads for the NAS to fetch ([1d70cd4](https://github.com/metaneutrons/ugos-cli/commit/1d70cd47af58b2c8430a69f6971597f508c85fd9))
* **fs:** browse and manage files on the NAS ([e4be161](https://github.com/metaneutrons/ugos-cli/commit/e4be16193eb104b3120a21206cd72eab7461b5ba))
* **fs:** upload and download files ([86151c8](https://github.com/metaneutrons/ugos-cli/commit/86151c8824653ae8b56af55b497b9bd71498082d))
* **log:** add the system log and user accounts ([2274f77](https://github.com/metaneutrons/ugos-cli/commit/2274f7765cc58068833c02c2bbae0bd9fdfe43e0))
* **system:** add machine info and live monitoring ([2d16e8e](https://github.com/metaneutrons/ugos-cli/commit/2d16e8ef15dffbf53c7904f3e5136daf3560d5fd))

## [0.6.0](https://github.com/metaneutrons/ugos-cli/compare/v0.5.0...v0.6.0) (2026-08-18)


### Features

* **storage:** report how many VMs each volume holds ([7da654e](https://github.com/metaneutrons/ugos-cli/commit/7da654e0da72522fdd98b309e676a9db89ef16c1))

## [0.5.0](https://github.com/metaneutrons/ugos-cli/compare/v0.4.0...v0.5.0) (2026-08-18)


### Features

* **image:** ISO upload from file or URL, plus a delete fix ([e409d26](https://github.com/metaneutrons/ugos-cli/commit/e409d26add87dbadf5dd9fe8aa75c83b08281d73))
* **image:** upload ISOs from a local file or a URL ([20e29de](https://github.com/metaneutrons/ugos-cli/commit/20e29de83dfb8d4b6cb1cba5811a9bebe675d16a))
* **kvm:** add overview, per-VM storage usage and PCI passthrough listing ([4022816](https://github.com/metaneutrons/ugos-cli/commit/402281658aa7ad499c71580bccddb2f3f9f044c8))
* **kvm:** explain failures using the web UI's validators ([12d0e7e](https://github.com/metaneutrons/ugos-cli/commit/12d0e7e0e8e1a3e860796effb69024fd50938923))


### Bug Fixes

* **auth:** report why a login was rejected ([0d90eed](https://github.com/metaneutrons/ugos-cli/commit/0d90eed29e3e689ba8f6c0490de2decd6fc36875)), closes [#3](https://github.com/metaneutrons/ugos-cli/issues/3)
* **image:** send the right parameter name when deleting an image ([647b2bd](https://github.com/metaneutrons/ugos-cli/commit/647b2bdc2555b54942f9f9949e1f0d0fe3541482))
* **snapshot:** use the parameters UGOS actually expects ([9e17b41](https://github.com/metaneutrons/ugos-cli/commit/9e17b414af9a6095cda5ccbb4e6cb60b63236fd2))

## [0.4.0](https://github.com/metaneutrons/ugos-cli/compare/v0.3.4...v0.4.0) (2026-08-17)


### Features

* **vm:** flexible create/update, plus KVM fixes verified on a live NAS ([9bffe0f](https://github.com/metaneutrons/ugos-cli/commit/9bffe0fef8fcf65a1e27fbda1c1e0e15a7262363))
* **vm:** give create and update the full device surface ([9cd74bd](https://github.com/metaneutrons/ugos-cli/commit/9cd74bd62307c09a8d48fff94f5e6718e9d9701b))


### Bug Fixes

* **client:** report API error codes instead of decode failures ([fca1b27](https://github.com/metaneutrons/ugos-cli/commit/fca1b27872728c57134ffab98b3cdb5a0207d030))
* **kvm:** correct the create and update request bodies ([0c66d44](https://github.com/metaneutrons/ugos-cli/commit/0c66d44cba7d94672ed9685bed822515e02b3525))
* **release:** publish directly from release please ([c9c39e1](https://github.com/metaneutrons/ugos-cli/commit/c9c39e13a812173e7962292580002d7990fc5191))

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
