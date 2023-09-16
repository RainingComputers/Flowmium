import setuptools

with open("README.md", "r") as f:
    long_description = f.read()

setuptools.setup(
    name="flowmium",
    version="0.0.1",
    author="RainingComputers",
    author_email="vishnu.vish.shankar@gmail.com",
    description="Workflow orchestrator for kubernetes.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/RainingComputers/Flowmium",
    packages=setuptools.find_packages(exclude=["docs", "tests"]),
    python_requires=">=3.8",
    install_requires=["requests"],
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Development Status :: 3 - Alpha",
    ],
    keywords="flowmium",
)
