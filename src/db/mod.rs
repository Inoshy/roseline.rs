extern crate diesel;

pub mod schema;
pub mod models;

use self::diesel::{
    sql_query,
    Connection
};
use self::diesel::sqlite::SqliteConnection;

pub use self::diesel::{
    result,
    RunQueryDsl,
    QueryDsl,
    ExpressionMethods,
    OptionalExtension,
    BelongingToDsl,
    TextExpressionMethods
};

use ::utils::ResultExt;

use ::rc::Rc;

#[derive(Clone)]
pub struct Db {
    inner: Rc<SqliteConnection>
}

impl Db {
    pub fn new() -> Result<Self, String> {
        let conn = SqliteConnection::establish("./roseline.db").format_err("To start DB")?;
        sql_query(schema::create::VNS).execute(&conn).format_err("create table")?;
        sql_query(schema::create::HOOKS).execute(&conn).format_err("create table")?;

        Ok(Self {
            inner: Rc::new(conn)
        })
    }

    pub fn delete_vn(&self, id: i64) -> result::QueryResult<usize> {
        debug!("DB: delete VN by id={}", id);
        use self::schema::vns::dsl;

        diesel::delete(dsl::vns.filter(dsl::id.eq(id))).execute(&*self.inner)
    }

    pub fn delete_hook(&self, vn: &models::Vn, version: &String) -> result::QueryResult<usize> {
        debug!("DB: delete for {:?} with version='{}'", vn, &version);
        use self::schema::hooks::dsl;

        diesel::delete(dsl::hooks.filter(dsl::vn_id.eq(&vn.id))
                                 .filter(dsl::version.like(version))).execute(&*self.inner)
    }
    pub fn put_hook(&self, vn: &models::Vn, version: String, code: String) -> result::QueryResult<models::HookView> {
        debug!("DB: put hook='{}' for version='{}'", code, version);
        use self::schema::hooks::dsl;
        let hook = models::Hook::belonging_to(vn).filter(dsl::version.like(&version))
                                                 .first::<models::Hook>(&*self.inner)
                                                 .optional()?;

        match hook {
            Some(hook) => {
                debug!("DB: found existing hook, update it");
                diesel::update(dsl::hooks.filter(dsl::id.eq(hook.id)))
                       .set(dsl::code.eq(&code))
                       .execute(&*self.inner).map(move |_| models::HookView { vn_id: hook.vn_id, version: hook.version, code: code })
            }
            None => {
                debug!("DB: adding new hook");
                let hook = models::HookView {
                    vn_id: vn.id,
                    version,
                    code
                };
                diesel::insert_into(dsl::hooks).values(&hook)
                                               .execute(&*self.inner).map(|_| hook)
            }
        }
    }

    ///Inserts VN if it is missing, or return existing one.
    pub fn put_vn(&self, id: i64, title: String) -> result::QueryResult<models::Vn> {
        use self::schema::vns::dsl;

        let vn = self.get_vn(id)?;

        match vn {
            Some(vn) => Ok(vn),
            None => {
                let vn = models::Vn { id, title };
                debug!("DB: put {:?}", &vn);

                diesel::insert_into(dsl::vns).values(&vn)
                                             .execute(&*self.inner).map(|_| vn)
            }
        }
    }

    #[inline]
    pub fn get_vn(&self, id: i64) -> result::QueryResult<Option<models::Vn>> {
        schema::vns::table.find(id).first::<models::Vn>(&*self.inner).optional()
    }

    #[inline]
    pub fn get_hooks(&self, vn: &models::Vn) -> result::QueryResult<Vec<models::Hook>> {
        models::Hook::belonging_to(vn).get_results(&*self.inner)
    }

    #[inline]
    pub fn count_vns(&self) -> result::QueryResult<i64> {
        Self::vns().count().get_result::<i64>(&*self.inner)
    }

    #[inline]
    pub fn count_hooks(&self) -> result::QueryResult<i64> {
        Self::hooks().count().get_result::<i64>(&*self.inner)
    }

    #[inline]
    ///Retrieves VN table
    pub fn vns() -> schema::vns::table {
        schema::vns::table
    }

    #[inline]
    ///Retrieves Hook table
    pub fn hooks() -> schema::hooks::table {
        schema::hooks::table
    }
}

impl ::std::ops::Deref for Db {
    type Target = SqliteConnection;

    fn deref(&self) -> &SqliteConnection {
        &self.inner
    }
}
