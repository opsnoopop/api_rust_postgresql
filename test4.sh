docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_4_constant_health_check.js

sleep 30

docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_4_constant_health_check.js

sleep 30

docker run \
--name container_k6 \
--rm \
-it \
--network global_rust \
-v ./k6/:/k6/ \
grafana/k6:1.1.0 \
run /k6/k6_4_constant_health_check.js
