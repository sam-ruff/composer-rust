### Basic example on how to use composer
Run the following: <br/>
```bash
composer install -i a-test-app
```

Get the application name with (has been manually set to 'a-test-app'):
```bash
composer list
``` 

Get the logs for that application <br/>
```bash
docker logs example_container
```

You should see:
```bash
> docker logs example_container
Logs for: test-application
Attaching to example_container
example_container | Hello, World.
```

Then uninstall the application:

```bash
composer delete a-test-app
``` 

Re-run with different values: (Note that order matters of the overrides) <br/>
```bash
composer install -i a-test-app -v values.yaml -v override.yaml
``` 
View the logs again, you should see a different message <br/>

```bash
> docker logs example_container
Logs for: test-application
Attaching to example_container
example_container | Hello again, with a different message.
```

Jinja is a very powerful templating language, you can easily add different services based on values.yaml or deploy different images into different environments etc.

### Templating config
You can also template config files, i.e. outside of template.yaml by using the configmap file extension. e.g. `my-json.configmap` and then using Jinja templating.