.PHONY: clean test lint docs requirements


clean:
	rm -f -r syncgit/__pycache__
	rm -f -r syncgit/zipdata/
	rm -f -r .pytest_cache/
	find . -type d -name  "__pycache__" -exec rm -r {} +
	rm -f -r .repos/
	make -C docs/ clean
	rm -f -r build/
	rm -f -r dist/

test:
	pytest tests -vv

format:
	black .

lint:
	flake8 flowmium tests
	mypy flowmium tests

docs:
	make -C docs/ clean
	make -C docs/ html
	open docs/_build/html/index.html

requirements:
	poetry export -f requirements.txt --output requirements.txt --with dev
