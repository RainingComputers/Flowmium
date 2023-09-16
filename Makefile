.PHONY: update-examples

update-examples:
	cp /home/shnoo/Projects/Flowmium/framework/tests/example_flow.py examples/python_package_workflow/my_flow
	cp /home/shnoo/Projects/Flowmium/framework/tests/example_flow.py examples/python_package_workflow

	mv examples/python_package_workflow/my_flow/example_flow.py  examples/python_package_workflow/my_flow/__main__.py
	mv  examples/python_package_workflow/example_flow.py examples/python_script_workflow/my_flow.py
	head flowmium/apidoc.http -n 55 | tail -n +4 | yq -y > examples/yaml_flow_definition/my_flow.yml

	cp flowmium/test-registries.yaml examples/python_package_workflow/