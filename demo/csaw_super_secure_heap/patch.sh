#!/bin/bash

patchelf ./bins/* --set-interpreter ./libs/ld-2.31.so
patchelf ./bins/* --set-rpath ./libs
