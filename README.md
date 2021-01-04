# Rauthy 🦖🛡️

Rauthy is a simple web service that requires a user to first authenticate using a query token, a header token or a basic auth username and password.
After they have authenticated their IP is stored and that IP will no longer require any authentication.

## Usage

Ensure your nginx configuration supports [subrequest authentication](https://docs.nginx.com/nginx/admin-guide/security-controls/configuring-subrequest-authentication/)

Download the latest release from [Github](https://github.com/Krakaw/rauthy/releases)

Set your .env

### Clients

There are multiple ways to authenticate against Rauthy

##### IP Auth

##### Basic Auth

##### Authorization TOKEN header

##### X-Bypass-Token TOKEN header

##### Query parameter ?token=TOKEN

##### Path parameter /path/parameters/TOKEN

```bash
curl https://raw.githubusercontent.com/Krakaw/rauthy/master/.env.sample -o .env
# Add a username and password
rauthy user -u username -p password
# Add an optional command to be run on successful auth
rauthy cmd -u username -c 'echo "my command"'
# Start the server
./rauthy
```

Configure nginx

```nginx
location /private {
    auth_request /auth;
    #...
}

location = /auth {
    internal;
    proxy_pass                          http://localhost:3031;
    proxy_pass_request_body             off;
    proxy_set_header                    Content-Length "";
    proxy_set_header X-Real-IP          $remote_addr;
    proxy_set_header X-Forwarded-For    $proxy_add_x_forwarded_for;
    proxy_pass_request_headers          on;
}
```

### Docker

```bash
docker build -t rauthy:latest .
docker-compose up
```

### Installation

```bash
git clone https://github.com/Krakaw/rauthy.git
cd rauthy
cp .env.sample .env
cargo run
```

## License
[MIT](https://choosealicense.com/licenses/mit/)
