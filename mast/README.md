## **MAST CONFIGURATION GENERATOR**

*MAST Configuration Generator* is a shell script that allows to merge all the
`.txt` configuration files inside the `/mast` directory in an unique file
(required by the *MAST tool*).   
In this way, it is possible to easily manage the configuration through a more
intuitive directory structure, avoiding a complex and error-prone unique file,
improving its maintainability.   

To organize the configuration each *task* has its own:
- scheduling_server.txt;
- operations.txt; 
- holistic_transactions.txt (both tasks and watchdog transactions);
- deadlines.txt (all operations and scheduling server watchdog).

Each *shared resource* instead has its own:
- shared_resource.txt;
- operations.txt.  

To provide offset-based analysis, all transactions for that purpose are defined
in the `offset_based_transactions.txt` file.

---
To use the MAST configuration generator is necessary to give the permission to
execute the script. 

```bash
chmod +x mast_generator.sh
```

Then, to generate the configuration file, run the script with the following command:

```bash
./mast_generator.sh
```
After selecting the desired model to generate based on the type of analysis
(holistic (1) or offset-based (2)), the resulting configuration files will be
saved inside the `/results` directory as `mast_holistic_configuration.txt` or
`mast_offset_based_configuration.txt`.
