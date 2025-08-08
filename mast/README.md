## **MAST CONFIGURATION GENERATOR**

*MAST Configuration Generator* is a shell script that allows to merge all the `.txt` configuration files inside the `/mast` directory in an unique file (required by the *MAST tool*).  
In this way, it is possible to easily manage the configuration through a more intuitive directory structure, avoiding a complex and error-prone unique file, improving its maintainability.  

To organize the configuration each *task* has its own:
- scheduling_server.txt;
- operations.txt; 
- transactions.txt;
- deadline.txt (deadline_scheduling_server, deadline_shared_resource and relative operations).

Each *shared resource* instead has its own:
- shared_resource.txt;
- operations.txt.  
---
To use the MAST configuration generator is necessary to give the permission to execute the script.

```bash
chmod +x mast_generator.sh
```

Then, to generate the configuration file, run the script with the following command:

```bash
./mast_generator.sh
```

The resulting configuration file will be saved in `mast_configuration.txt`.
