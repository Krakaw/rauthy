# Nginx Auth

Nginx Auth is a simple web server that requires an IP to first enter basic auth information,
after initial auth is complete, that IP no longer requires to send the basic auth headers with each request.

## Usage

Ensure your nginx configuration supports [subrequest authentication](https://docs.nginx.com/nginx/admin-guide/security-controls/configuring-subrequest-authentication/)

Download the latest release from [Github](https://github.com/Krakaw/nginx-auth/releases)

Set your .env

```bash
curl https://raw.githubusercontent.com/Krakaw/nginx-auth/master/.env.sample -o .env
./nginx-auth
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
docker build -t nginx-auth:latest .
docker-compose up
```

### Installation

```bash
git clone https://github.com/Krakaw/nginx-auth.git
cd nginx-auth
cp .env.sample .env
cargo run
```

## License
[MIT](https://choosealicense.com/licenses/mit/)
