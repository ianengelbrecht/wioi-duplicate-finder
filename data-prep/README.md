## Preparing a new dataset for the Duplicate Finder

The Duplicate Finder application allows you to swap out the existing/default reference dataset with a new dataset for a different region, or an updated dataset for your current region. In the application, under the Settings tab on the home screen, you can see the current countries and herbaria that are included in your reference dataset.

The reference data are prepared from the data available on GBIF. The first step is to go to GBIF to download the appropriate data. Filter the data for vascular plants (Tracheophyta), the countries you are working with (usually your home country and neighbours), and the primary datasets that will have duplicate records for your collection. You might want to spend some time browsing the available datasets first and recording the dataset names to use when you set up the relevant filters.

GBIF data can be quite messy, so we need to spend some time sorting things out first. In particular, we need to clean the collectors field and the collectionCode field. The diagram below shows how to prepare the data for import into the application. The accompanying Python scripts will assist in this process. If you're working with large numbers of records, the most work is generating the cleaned collectors list.
