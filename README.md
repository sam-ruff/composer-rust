# Composer
Composer is a command line tool similar to Helm, but designed for Docker Compose. It enables templating for Docker Compose files, making it easy to manage and deploy applications with different configurations.

## Features
- Install, upgrade, and delete applications using Jinja2 templates
- List installed Composer applications
- Print the output docker-compose.yaml after values have been applied
- Support for application configuration through values.yaml and app.yaml files
- Automatically pull images before installing or upgrading an application (optional)

## Prerequisites
Docker
Docker Compose

## Getting Started
To install Composer, run the following command for Linux:
```bash
curl -fsSL https://raw.githubusercontent.com/ByteSquid/composer-rust/master/scripts/install-linux.sh | bash
```
For AWS Linux:
```bash
curl -fsSL https://raw.githubusercontent.com/ByteSquid/composer-rust/master/scripts/install-aws.sh | bash
```
For other platforms, clone the repository and build the binary:
## Binaries
The latest releases for OSX, Linux and Windows can be found here.
```
https://github.com/ByteSquid/composer-rust/releases
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
git clone https://github.com/ByteSquid/composer-rust.git
cd composer-rust
cargo build --release
```
Copy the binary to a location in your PATH, e.g. `/usr/local/bin`:
```bash
sudo cp target/release/composer /usr/local/bin
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

## Contributing
Contributions are welcome! Please submit a pull request or create an issue to discuss any changes.