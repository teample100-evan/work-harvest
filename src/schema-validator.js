import { readFile } from "node:fs/promises";
import path from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { packageRoot } from "./paths.js";

function formatErrors(errors) {
  return (errors ?? []).map((error) => {
    const location = error.instancePath || "/";
    return `${location} ${error.message}`;
  });
}

export async function createSchemaValidators() {
  const schemaDirectory = path.join(packageRoot, "schemas");
  const [common, workItem, workContext, checkpoint] = await Promise.all(
    ["common", "work-item", "work-context", "checkpoint"].map(async (name) =>
      JSON.parse(
        await readFile(path.join(schemaDirectory, `${name}.schema.json`), "utf8"),
      ),
    ),
  );

  const ajv = new Ajv2020({
    allErrors: true,
    allowUnionTypes: true,
    strict: true,
  });
  addFormats(ajv);
  ajv.addSchema(common);

  const validateWorkItemSchema = ajv.compile(workItem);
  const validateWorkContextSchema = ajv.compile(workContext);
  const validateCheckpointSchema = ajv.compile(checkpoint);

  return {
    workItem(value) {
      const valid = validateWorkItemSchema(value);
      return { valid, errors: formatErrors(validateWorkItemSchema.errors) };
    },
    workContext(value) {
      const valid = validateWorkContextSchema(value);
      return { valid, errors: formatErrors(validateWorkContextSchema.errors) };
    },
    checkpoint(value) {
      const valid = validateCheckpointSchema(value);
      return { valid, errors: formatErrors(validateCheckpointSchema.errors) };
    },
  };
}
