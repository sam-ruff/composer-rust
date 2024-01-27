
# Composer
Composer is a command line tool similar to Helm, but designed for Docker Compose. It enables templating for Docker Compose files, making it easy to manage and deploy applications with different configurations.

## Status
[![Release](https://github.com/ByteSquid/composer-rust/actions/workflows/release.yml/badge.svg)](https://github.com/ByteSquid/composer-rust/actions/workflows/release.yml)
<br/>
## Features
- Install, upgrade, and delete applications using Jinja2 templates
- List installed Composer applications
- Print the output docker-compose.yaml after values have been applied
- Support for application configuration through values.yaml and app.yaml files
- Automatically pull images before installing or upgrading an application (optional)

## Prerequisites
- Docker
- Docker Compose (specifically `docker compose` not `docker-compose` python plugin, so you need a relatively up-to-date version of docker).
- jq for installation script

## Getting Started
To install Composer, run the following command for Ubuntu/Rocky:
```bash
curl -fsSL https://raw.githubusercontent.com/ByteSquid/composer-rust/master/scripts/install-ubuntu.sh | bash
```
For AWS Linux 2 or anything linux-based not using glibc:
```bash
curl -fsSL https://raw.githubusercontent.com/ByteSquid/composer-rust/master/scripts/install-musl.sh | bash
```
For other platforms, clone the repository and build the binary.
## Binaries
The latest releases for OSX, Linux and Windows can be found here.
```
https://github.com/ByteSquid/composer-rust/releases
```

## RPM Installation
Add Package Cloud Repos
### Debian
```bash
curl -s https://packagecloud.io/install/repositories/sam-bytesquid/composer-production/script.deb.sh | sudo bash
```
### RPM Other
```bash
curl -s https://packagecloud.io/install/repositories/sam-bytesquid/composer-production/script.rpm.sh | sudo bash
```
Then do (replacing for the latest version):
```bash
sudo yum install composer-1.21-1.x86_64
```
## Building from Source
Note: You will need rust installed also for this.
https://www.rust-lang.org/tools/install 
or 
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Then get the composer source:
```bash
source "$HOME/.cargo/env"
git clone https://github.com/ByteSquid/composer-rust.git
cd composer-rust
cargo build --release
```
Copy the binary to a location in your PATH, e.g. `$HOME/.local/bin`:
```bash
mkdir -p 
cp target/release/composer $HOME/.local/bin
# If you dont have /usr/local/bin on your path
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
# You can then verify with the command
composer --version
```

## Usage
The basic syntax for Composer is:
```bash
composer [global flags] command [flags] [arguments]
```

### Global Flags
* `-l, --log_level <LOG_LEVEL>`: Set the verbosity level. Possible values are INFO, ERROR, TRACE, WARN. Default is INFO.

* `-p, --always_pull`: If set, Composer will attempt to pull all images specified in the template.jinja file before installing or upgrading an application.

### Commands
* `install, i, add`: Install a Docker Compose application using a given Jinja2 template.
* `upgrade, u, update`: Upgrade an existing Composer application. This is equivalent to running docker-compose up again. Existing services will remain, and only the differences will be applied.
* `list, ls, ps`: List installed Composer applications.
* `template, t`: Print the output docker-compose.yaml after values have been applied. This can be used to produce a Compose file for use outside of the Composer install environment or for debugging purposes.
* `delete, d, uninstall`: Delete a given application(s) (by ID unless using --all), removing it completely.


## Configuration
Composer relies on several configuration files for templating and application settings:

`app.yaml`: Contains application metadata such as name, version, and description. <br/>
`docker-compose.jinja2`: A Jinja2 template for the docker-compose.yaml file. <br/>
It will also template any other files that have extensions `.jinja2` <br/>

## Example
In the `resources/example_app` directory, you'll find a sample application with the necessary configuration files. To install this application, run: 
```bash
composer install resources/example_app -v resources/example_app/values.yaml -i example
```
To see a list of installed applications, run:
```bash
composer list
```
You should see something like this:
```bash
> composer list
APP ID          VERSION         UPTIME          STATUS          APP NAME                  COMPOSE             
example         1.0.0           now             RUNNING         simple-app                resources/example_app
```
You can view the running container logs with:
```bash
> docker logs example_container
Hello, World.
```
To upgrade the example application, modify the values.yaml or override.yaml files, and run:
```bash
composer upgrade -i example -v resources/example_app/values.yaml -v resources/example_app/override.yaml resources/example_app
```
You can then grab the logs of the container to see the overriden variable:
```bash
> docker logs example_container
Hello again, with a different message.
```
To delete the application: 
```bash
composer delete example
```
## Nested Compose files
You can nest docker-compose.jinja2 files in sub-directories and they will be started up as a single app. This is useful for managing complex deployments as a single unit globally.
## Composer Ignore
When you do `composer install` the working directory is copied into `~/.composer/` and the templates are applied. If you don't want certain unnecessary files to be copied such as large files. 
Add them to a file at the root `.composerignore`. This has the same syntax as `.dockerignore` files.
## Templating config
All files with the file extension `.jinja2` will be templated. This is useful for also templating config files etc. that are going to be mounted into a container.
We recommend using a pattern such as the following (using nginx config as an example):
```yaml
version: "3.9"
services:
  frontend:
    restart: unless-stopped
    container_name: nginx
    image: {{ registry }}/{{ nginx.image }}:{{ nginx.image_version }}
    volumes:
      - type: bind
        source: config.jinja2
        target: /usr/share/nginx/html/config/config.json
```
In this example a templated config file is mounted in as `.json` so that its picked up correctly post-templating. This can be very powerful when switching between environments.
## Contributing
Contributions are welcome! Please submit a pull request or create an issue to discuss any changes.
