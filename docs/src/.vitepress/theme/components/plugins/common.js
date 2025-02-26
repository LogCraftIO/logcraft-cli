export const getCategoryColor = (stage) => {
  switch (stage.toLowerCase()) {
    case "siem":
      return "teal";
    case "xdr":
      return "blue";
    case "edr":
      return "indigo";
    case "format":
      return "purple";
    default:
      return "grey";
  }
};
