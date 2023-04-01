# Manual e2e testing

```
docker build . -t framework-testing
docker tag framework-testing localhost:5000/framework-testing:latest
docker push localhost:5000/framework-testing:latest
python3 test.py --image registry:5000/framework-testing:latest --cmd python3 test.py
```
