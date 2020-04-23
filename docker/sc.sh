sudo docker build -t python-app .
sudo docker run --detach --name contname  --rm -t python-app
docker stop contname
sudo docker inspect pa | grep "IPAddress":
sudo docker inspect -f '{{.Name}} - {{.NetworkSettings.IPAddress }}' $(sudo docker ps -aq)
